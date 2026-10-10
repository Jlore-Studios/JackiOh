//! §10.5 steps 6 and 7: an Echo repeat, the state check after a card's last resolution, and the traps
//! that answer it (SPEC §4.5, §10.5, R17, R59, R61, R174).
//!
//!  - §10.5 step 6 is "repeat step 5", and step 5 is the granted Combo parts (#38, #78) and then the
//!    card's own script, so a repeat runs all three.
//!  - §4.5, R59: the check runs after a card's whole Cry or spell, before step 7's `cardResolved`, so
//!    #60 and #85 meet the board the card left.
//!  - R174, R61: the traps answering one play fire one after another; once an earlier one has taken
//!    the played card off the field, the next meets a play that is no longer in play.
//!  - §4.5, R118: step 4's loop runs no state check before anything has resolved, so a card that
//!    arrives at 0 or less health still resolves its Cry and dies in the check after it.
//!  - R174: the play follows the stay step 4 put the card on (a unit back through Reborn is a new
//!    arrival, not in play at step 7; R1, R118, R61), and its targets the stays step 1 checked.

use jackioh_engine::testkit::*;

const BIGOT: &str = "core-002";
const RIGHT_HOUSE: &str = "core-003";
const SEVEN_SEVEN: &str = "core-025";
const SUPPRESSIVE_AURA: &str = "core-046";
const GARY: &str = "core-004";
const STOCKPILE: &str = "core-005";
const VANILLA: &str = "core-008";
const TIMMY: &str = "core-011";
const MR_TOKEN: &str = "core-015";
const LUNAR_ECLIPSE: &str = "core-035";
const QUICKSTRIKER: &str = "core-038";
const BIG_FELINOR: &str = "core-043";
const RENO: &str = "core-053";
const HONEYPOT: &str = "core-060";
const FULLSEND: &str = "core-078";
const TWINSPELL: &str = "core-079";
const UNLICENSED: &str = "core-085";
const RUSH_TOKEN: &str = "core-t-rush";
const LIBRARY: [&str; 8] = [
    VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA,
];

/// The harness with the real catalog and every card script registered: the engine's testkit cannot
/// name the cards crate, so the cards test does.
fn setup(opts: Value) -> Scenario {
    jackioh_cards::register_all();
    scenario(opts)
}

fn at(card: &CardInstance) -> Value {
    json!([{ "pick": "instance", "instanceId": card.id }])
}

fn unit_at(g: &Scenario, player: &'static str, lane: i32) -> CardInstance {
    g.unit(player, lane)
        .unwrap_or_else(|| panic!("setup: {player} should hold a unit in lane {lane}"))
}

fn count(events: &[GameEvent], type_: GameEventType) -> usize {
    events.iter().filter(|event| event.event_type() == type_).count()
}

/// §10.5 step 6: an Echo repeat is step 5 again, granted Combo parts included
mod s10_5_step_6_an_echo_repeat_is_step_5_again_granted_combo_parts_included {
    use super::*;

    #[test]
    fn r30_s10_5_quickstrikers_granted_combo_hits_once_per_resolution_of_an_echoed_spell_s8_c38() {
        // Twinspell is the one card played earlier, so X = 1 on each of Stockpile's two resolutions.
        let mut g = setup(json!({
            "p1": { "hand": [TWINSPELL, STOCKPILE, RENO], "backrow": [{ "def": QUICKSTRIKER, "lane": 2 }], "mana": 8, "library": LIBRARY },
            "p2": { "hand": [RENO], "library": LIBRARY },
        }));
        g.play(TWINSPELL, json!({ "zone": 1 }));
        let before = g.hand("p1").len();

        g.play(STOCKPILE, json!({}));

        // Stockpile's own text ran twice (draw 2, twice), and Quickstriker's Combo with it.
        assert_eq!(g.hand("p1").len(), before - 1 + 4);
        let hits = g
            .last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Damage { target_id, .. } if target_id == "hero-p2"))
            .count();
        assert_eq!(hits, 2);
        g.expect_health("p2", 28);
    }

    #[test]
    fn r30_s10_5_fullsends_granted_combo_draw_1_draws_once_per_resolution_of_an_echoed_spell_s8_c78() {
        // A Radiant /fullsend (its face grants the Combo draw) and Twinspell were played earlier: each
        // of Stockpile's two resolutions draws 1 for the granted Combo and then 2 for Stockpile.
        let mut g = setup(json!({
            "p1": { "hand": [{ "def": FULLSEND, "radiant": true }, TWINSPELL, STOCKPILE, RENO], "mana": 8, "library": LIBRARY },
            "p2": { "hand": [RENO], "library": LIBRARY },
        }));
        g.play(FULLSEND, json!({}));
        g.play(TWINSPELL, json!({ "zone": 1 }));
        let before = g.hand("p1").len();

        g.play(STOCKPILE, json!({}));

        let drawn = g
            .last_events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    GameEvent::Drawn {
                        player: PlayerId::P1,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(drawn, 6);
        assert_eq!(g.hand("p1").len(), before - 1 + 6);
    }
}

/// §4.5: the check after a card's whole Cry or spell, before step 7's traps
mod s4_5_the_check_after_a_cards_whole_cry_or_spell_before_step_7s_traps {
    use super::*;

    #[test]
    fn r59_s4_5_radiant_bear_honeypot_fills_the_zones_big_felinors_cry_has_just_emptied_s10_5_step_7() {
        let mut g = setup(json!({
            "p1": { "hand": [BIG_FELINOR, STOCKPILE], "library": LIBRARY },
            "p2": {
                "hand": [STOCKPILE],
                "field": [{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 2 }],
                "backrow": [{ "def": HONEYPOT, "radiant": true, "faceUp": false }],
                "library": LIBRARY,
            },
        }));

        // Big Felinor's Cry destroys every non-Felinor unit, and the check after the whole Cry collects
        // both Mr. Vanillas; only then does `cardResolved` reach the trap, whose "fill your board" finds
        // all five of p2's unit zones empty.
        g.play(BIG_FELINOR, json!({ "zone": 1 }));

        let tokens = g
            .events()
            .iter()
            .filter(|e| {
                matches!(e, GameEvent::Summoned { def_id, player: PlayerId::P2, .. } if def_id == RUSH_TOKEN)
            })
            .count();
        assert_eq!(tokens, 5);
    }

    #[test]
    fn r61_r99_s4_5_unlicensed_experimentation_does_not_fuse_big_felinor_onto_a_unit_its_cry_has_already_destroyed()
     {
        let mut g = setup(json!({
            "p1": { "hand": [BIG_FELINOR, STOCKPILE], "library": LIBRARY },
            "p2": {
                "hand": [STOCKPILE],
                "field": [{ "def": TIMMY, "lane": 1 }],
                "backrow": [{ "def": UNLICENSED, "faceUp": false }],
                "library": LIBRARY,
            },
        }));
        let timmy = unit_at(&g, "p2", 1);

        g.play(BIG_FELINOR, json!({ "zone": 1 }));

        // Timmy died with the Cry, so p2 controls no Unit when the trap is asked, and it stays armed.
        g.expect_in_zone(&timmy, "graveyard");
        assert_eq!(count(g.events(), GameEventType::TrapFired), 0);
        assert_eq!(g.unit("p1", 1).map(|c| c.def_id), Some(BIG_FELINOR.to_string()));
    }

    #[test]
    fn r59_r64_s4_5_base_bear_honeypot_summons_into_the_zone_lunar_eclipses_spell_has_just_emptied() {
        let mut g = setup(json!({
            "p1": { "hand": [LUNAR_ECLIPSE, STOCKPILE], "library": LIBRARY },
            "p2": {
                "hand": [STOCKPILE],
                "field": [VANILLA, VANILLA, { "def": VANILLA, "damage": 1 }, VANILLA, VANILLA],
                "backrow": [{ "def": HONEYPOT, "faceUp": false }],
                "library": LIBRARY,
            },
        }));
        let victim = unit_at(&g, "p2", 3);

        // 3 damage kills the lane-3 Mr. Vanilla as the spell ends, so its zone is empty by step 7:
        // "summon 2 Rush Tokens" takes that one zone and the second summon fails in silence (§3.2).
        g.play(LUNAR_ECLIPSE, json!({ "targets": at(&victim) }));

        g.expect_in_zone(&victim, "graveyard");
        assert_eq!(count(g.events(), GameEventType::TrapFired), 1);
        assert_eq!(g.unit("p2", 3).map(|c| c.def_id), Some(RUSH_TOKEN.to_string()));
    }
}

/// R174: a trap after Bear Honeypot's run meets the played unit the run killed as gone
mod r174_a_trap_after_bear_honeypots_run_meets_the_played_unit_the_run_killed_as_gone {
    use super::*;

    #[test]
    fn r174_r61_unlicensed_experimentation_does_not_fuse_a_played_unit_bear_honeypots_tokens_already_killed_out_of_the_graveyard()
     {
        let mut g = setup(json!({
            "active": "p1",
            "p1": { "hand": [MR_TOKEN, STOCKPILE], "library": LIBRARY },
            "p2": {
                "hand": [STOCKPILE],
                "field": [{ "def": GARY, "lane": 5 }],
                "backrow": [
                    { "def": HONEYPOT, "faceUp": false, "lane": 1 },
                    { "def": UNLICENSED, "faceUp": false, "lane": 2 },
                ],
                "library": LIBRARY,
            },
        }));

        // Me and Mr Token (1/1, cost 1) resolves; step 7's `cardResolved` reaches p2's traps in lane
        // order (R68). Bear Honeypot's first Rush Token kills it, so when Unlicensed Experimentation is
        // asked, the play it would fuse onto p2's side is in p1's graveyard.
        g.play(MR_TOKEN, json!({ "zone": 1 }));

        assert!(
            g.pile("p1", "graveyard")
                .iter()
                .any(|card| card.def_id == MR_TOKEN)
        );
        assert_eq!(count(g.events(), GameEventType::Fused), 0);
    }

    #[test]
    fn r174_r83_a_second_bear_honeypots_tokens_do_not_attack_the_reborn_body_of_the_unit_the_first_ones_run_killed()
     {
        let mut g = setup(json!({
            "active": "p1",
            "p1": { "hand": [RIGHT_HOUSE, STOCKPILE], "library": LIBRARY },
            "p2": {
                "hand": [STOCKPILE],
                "backrow": [
                    { "def": HONEYPOT, "faceUp": false, "lane": 1 },
                    { "def": HONEYPOT, "faceUp": false, "lane": 2 },
                ],
                "library": LIBRARY,
            },
        }));

        // Right-house defender (1/1, Taunt, Divine Shield, Reborn): the first trap's first token takes
        // its shield and the second kills it, and Reborn brings it straight back. The second trap still
        // fires — the play cost 1 — but "they attack it" named that play's stay, which has ended.
        g.play(RIGHT_HOUSE, json!({ "zone": 1 }));

        assert_eq!(count(g.events(), GameEventType::TrapFired), 2);
        assert_eq!(count(g.events(), GameEventType::AttackDeclared), 2);
        let defender = g.card(RIGHT_HOUSE).clone();
        g.expect_in_zone(&defender, "field");
    }
}

/// §4.5, R118: a played unit that does not survive its own arrival still resolves its Cry
mod s4_5_r118_a_played_unit_that_does_not_survive_its_own_arrival_still_resolves_its_cry {
    use super::*;

    #[test]
    fn r118_bigot_played_under_suppressive_aura_destroys_its_target_before_it_dies_s10_5_steps_4_5_s4_5() {
        let mut g = setup(json!({
            "p1": { "hand": [BIGOT, STOCKPILE], "backrow": [{ "def": SUPPRESSIVE_AURA, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [STOCKPILE], "field": [{ "def": SEVEN_SEVEN, "lane": 1 }], "library": LIBRARY },
        }));
        let prey = unit_at(&g, "p2", 1);
        let bigot = g.card(BIGOT).clone();

        // Bigot is 6/1 printed, 4/-1 under the aura. No trap answered it and nothing took it off the
        // field at step 4, so step 5 resolves its Cry (R118: the Cry is lost only where a trap has taken
        // the card off the field), and the check after the play collects it (§4.5).
        g.play(BIGOT, json!({ "targets": at(&prey) }));
        g.expect_in_zone(&prey, "graveyard");
        g.expect_in_zone(&bigot, "graveyard");
    }
}

// The play follows the stay step 4 put the card on, and its choices the stays step 1 checked (R174)

const POINTMASTER: &str = "core-020";
const COLLATERAL_DAMAGE: &str = "core-034";
const PLASTIC_SURGERY: &str = "core-063";

/// A fixture card: a transient def in the match state and its script in the registry.
///
/// `face` is `{ attack?, health?, keywords? }` (`{}` for none); a Unit's printed stats default to 2/2
/// and its keywords to none.
fn fixture_card(state: &mut GameState, id: &str, type_: &str, script: Script, face: Value) {
    let keywords = face.get("keywords").cloned().unwrap_or_else(|| json!([]));
    let printed = if type_ == "Unit" {
        json!({
            "attack": face.get("attack").cloned().unwrap_or_else(|| json!(2)),
            "health": face.get("health").cloned().unwrap_or_else(|| json!(2)),
            "keywords": keywords,
            "text": id,
        })
    } else {
        json!({ "keywords": keywords, "text": id })
    };
    let def: CardDef = json_as(json!({
        "id": id,
        "index": id,
        "name": id,
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": printed.clone(),
        "radiant": printed,
    }));
    state.transient_defs.insert(id.to_string(), def);
    let mut all: IndexMap<String, CardScripts> = scripts::registered_scripts().clone();
    all.insert(
        id.to_string(),
        CardScripts {
            base: script.clone(),
            radiant: script,
        },
    );
    register_scripts(all);
}

fn in_hand_of(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    let card = new_instance(state, def_id, player, Zone::Hand { player });
    state.players[player].hand.push(card.clone());
    card
}

fn face_down_trap(state: &mut GameState, def_id: &str, player: PlayerId, lane: i32) -> CardInstance {
    let mut card = new_instance(state, def_id, player, Zone::Hand { player });
    let slot = ZoneRef {
        player,
        row: Row::Backrow,
        lane,
    };
    if !zones::place_on_field(state, &mut card, slot, Default::default()) {
        panic!("could not place {def_id}");
    }
    let live = find_instance_mut(state, &card.id).unwrap_or_else(|| panic!("could not place {def_id}"));
    live.face_up = Some(false);
    live.clone()
}

/// A trap that destroys the Unit its controller's opponent plays, at §10.5 step 4 as Sheepish answers it.
fn slay_on_play() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("edge-r7-slay", &[GameEventType::CardPlayed], |_ctx, event| match event {
                GameEvent::CardPlayed { instance_id, .. } => vec![effects::destroy(json_as(json!({
                    "target": { "of": "instance", "instanceId": instance_id }
                })))],
                _ => vec![],
            })
            .with_when(|ctx, event| matches!(event, GameEvent::CardPlayed { player, .. } if *player != ctx.controller)),
        ],
        ..Script::default()
    }
}

/// A Field Trap that answers every play of the opponent's by destroying every unit (step 4, R17).
fn wipe_on_play() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("edge-r7-wipe-on-play", &[GameEventType::CardPlayed], |_ctx, _event| {
                vec![effects::destroy_all(json_as(json!({})))]
            })
            .with_when(|ctx, event| matches!(event, GameEvent::CardPlayed { player, .. } if *player != ctx.controller)),
        ],
        ..Script::default()
    }
}

/// R174, R118, R61: the play follows the stay step 4 put the card on, not a Reborn body
mod r174_r118_r61_the_play_follows_the_stay_step_4_put_the_card_on_not_a_reborn_body {
    use super::*;

    #[test]
    fn r174_r118_r61_a_played_reborn_unit_a_step_4_trap_kills_comes_back_through_reborn_and_neither_its_cry_nor_unlicensed_experimentation_treats_the_body_as_the_card_being_played_s4_5_step_4()
     {
        let mut g = setup(json!({
            "p1": { "hand": [STOCKPILE], "mana": 4 },
            "p2": { "hand": [STOCKPILE], "field": [{ "def": GARY, "lane": 1 }], "backrow": [{ "def": UNLICENSED, "lane": 2 }] },
        }));
        // A Unit with Reborn whose Cry deals 3 to the enemy hero, and a trap that destroys a Unit the
        // opponent plays. No Core trap removes a played unit and lets it come back, so both are fixtures.
        fixture_card(
            g.state_mut(),
            "edge-r7-reborn-cry",
            "Unit",
            Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "enemyHero" }, "amount": 3 }),
                    ))]
                })),
                ..Script::default()
            },
            json!({ "keywords": [{ "kind": "Reborn" }] }),
        );
        fixture_card(g.state_mut(), "edge-r7-slay", "Trap", slay_on_play(), json!({}));
        face_down_trap(g.state_mut(), "edge-r7-slay", PlayerId::P2, 1);
        let unit = in_hand_of(g.state_mut(), "edge-r7-reborn-cry", PlayerId::P1);

        g.play(&unit, json!({ "zone": 3 }));

        // The trap killed the played unit at step 4 and Reborn brought it straight back (§4.5 step 4): a
        // reset instance that entered the field again (R78, R83). R118, R17: the trap took the played
        // card off the field, so its Cry is lost. R61: a Reborn result never sets #85 off, which meets
        // the play as no longer in play (R174), so the body stays the fixture on p1's side, not fused.
        let body = find_instance(g.state(), &unit.id).map(|body| {
            format!(
                "{}:{}:{}",
                body.zone.z().as_str(),
                body.def_id,
                if body.reborn_spent == Some(true) {
                    "reborn"
                } else {
                    "first"
                }
            )
        });
        let died = g.events().iter().any(
            |event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == unit.id),
        );
        assert_eq!(
            json!({
                "died": died,
                "body": body.unwrap_or_else(|| "gone".to_string()),
                "p2Health": g.state().players.p2.hero.health,
                "fused": g.events().iter().any(|event| event.event_type() == GameEventType::Fused),
            }),
            json!({ "died": true, "body": "field:edge-r7-reborn-cry:reborn", "p2Health": 30, "fused": false })
        );
    }

    #[test]
    fn r174_r53_r83_bear_honeypots_tokens_do_not_attack_the_reborn_body_of_a_played_unit_that_died_in_its_own_resolution_s10_5_step_7()
     {
        // p1 plays a 0-cost Reborn unit whose Cry kills itself. Step 6's check collects it and Reborn
        // brings a new arrival back at 1 health (§4.5 step 4, R83) before step 7. p2's Bear Honeypot
        // answers the play: "if it was a Unit, they attack it" — but the unit that was played has left
        // the field, so the tokens attack nothing and the Reborn body is left standing.
        let mut s = setup(json!({
            "seed": "edge-r7-honeypot-martyr",
            "p1": { "hand": [STOCKPILE], "library": LIBRARY },
            "p2": { "backrow": [{ "def": HONEYPOT, "faceUp": false }], "hand": [STOCKPILE], "library": LIBRARY },
        }));
        fixture_card(
            s.state_mut(),
            "edge-r7-martyr",
            "Unit",
            Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "self" }, "amount": 9 }),
                    ))]
                })),
                ..Script::default()
            },
            json!({ "attack": 3, "health": 3, "keywords": [{ "kind": "Reborn" }] }),
        );
        let martyr = in_hand_of(s.state_mut(), "edge-r7-martyr", PlayerId::P1);

        s.play(&martyr, json!({ "zone": 3 }));

        // The play died and came back: the Reborn body stands in lane 3.
        assert!(s.events().iter().any(
            |event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == martyr.id)
        ));
        assert!(
            s.events()
                .iter()
                .any(|event| matches!(event, GameEvent::TrapFired { def_id, .. } if def_id == HONEYPOT))
        );
        let forced: Vec<&GameEvent> = s
            .events()
            .iter()
            .filter(|event| matches!(event, GameEvent::AttackDeclared { forced: true, .. }))
            .collect();
        assert!(forced.is_empty(), "forced attacks: {forced:?}");
        assert_eq!(s.unit("p1", 3).map(|c| c.id), Some(martyr.id.clone()));
        assert_eq!(s.card(&martyr).reborn_spent, Some(true));
    }
}

/// R174: a target a trap answering the play took off the field is gone for the play's step 5
mod r174_a_target_a_trap_answering_the_play_took_off_the_field_is_gone_for_the_plays_step_5 {
    use super::*;

    #[test]
    fn r174_r78_c63_plastic_surgery_does_not_buff_or_grant_a_keyword_to_its_target_in_the_graveyard_after_a_step_4_trap_destroyed_it_s8_conventions()
     {
        let mut s = setup(json!({
            "seed": "r7-surgery-on-the-dead",
            "p1": { "hand": [PLASTIC_SURGERY], "mana": 4 },
            "p2": { "field": [POINTMASTER] },
        }));
        fixture_card(
            s.state_mut(),
            "edge-r7-wipe",
            "Field Trap",
            wipe_on_play(),
            json!({}),
        );
        face_down_trap(s.state_mut(), "edge-r7-wipe", PlayerId::P2, 5);
        let target = unit_at(&s, "p2", 1);

        s.play(PLASTIC_SURGERY, json!({ "targets": at(&target) }));

        // The trap fired at step 4 and its check collected the Pointmaster (§4.5, R17).
        assert!(s.last_events().iter().any(
            |event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == target.id)
        ));
        assert!(s.pile("p2", "graveyard").iter().any(|card| card.id == target.id));
        // §8 Conventions: the spell's target is gone, so its text fizzles on it; R78: a card in a
        // graveyard is the printed card, with no buffs and no granted keywords.
        let dead = s.card(&target.id).clone();
        assert_eq!(dead.buffs, AttackHealth { attack: 0, health: 0 });
        assert_eq!(dead.granted_keywords, Vec::<Keyword>::new());
        let touched: Vec<&GameEvent> = s
            .last_events()
            .iter()
            .filter(|event| match event {
                GameEvent::Buffed { instance_id, .. } | GameEvent::KeywordGranted { instance_id, .. } => {
                    *instance_id == target.id
                }
                _ => false,
            })
            .collect();
        assert!(
            touched.is_empty(),
            "buffs or grants on the dead target: {touched:?}"
        );
    }

    #[test]
    fn r174_r55_c34_collateral_damage_does_not_exile_its_target_out_of_the_graveyard_after_a_step_4_trap_destroyed_it_s8_conventions()
     {
        let mut s = setup(json!({
            "seed": "r7-collateral-on-the-dead",
            "p1": { "hand": [COLLATERAL_DAMAGE], "mana": 4 },
            "p2": { "field": [POINTMASTER], "library": [TIMMY, TIMMY] },
        }));
        fixture_card(
            s.state_mut(),
            "edge-r7-wipe",
            "Field Trap",
            wipe_on_play(),
            json!({}),
        );
        face_down_trap(s.state_mut(), "edge-r7-wipe", PlayerId::P2, 5);
        let target = unit_at(&s, "p2", 1);

        s.play(COLLATERAL_DAMAGE, json!({ "targets": at(&target) }));

        // The trap's wipe put the Pointmaster in its owner's graveyard (§4.5, R17), and there it stays:
        // the permanent the spell named is gone, so only the library half of its text resolves.
        assert!(s.last_events().iter().any(
            |event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == target.id)
        ));
        assert!(!s.pile("p2", "exile").iter().any(|card| card.id == target.id));
        assert!(s.pile("p2", "graveyard").iter().any(|card| card.id == target.id));
    }
}
