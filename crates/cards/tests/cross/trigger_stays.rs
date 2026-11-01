//! An event is answered as the board stood when it happened (SPEC §10.3, §4.5, R174, R212).
//! Paths and testkit: docs/v0.3.0/SURFACE.md §4.1, §8.
//! §10.3's loop hands an event to the triggers after whatever ran before it was dispatched: a state
//! check (with its Death hooks and Reborn) after a combat, an Echo repeat or a whole Cry. So #91 Fed
//! Fauci or #32 Prem Panther, offered its event, can be a Reborn body, a card drawn since, or a unit
//! a Death has stolen since. R212: a card that has moved zones since is on a stay that did not see
//! the event, and a card whose controller changed answers for the player who held it then. The
//! traps, an event's first responders, follow the same rule. R174: a response the loop hands an
//! event later is aimed at the stay its event's card had then, not a Reborn body back since; every
//! other card it reads off the board is on the stay it has then. A fixture stands in where no Core
//! card steals a trap, casts a Unit or aims a trigger at its event's card.

use std::sync::Arc;

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

const GARY: &str = "core-004";
const STOCKPILE: &str = "core-005";
const VANILLA: &str = "core-008";
const MOTHS: &str = "core-009";
const PANTHER: &str = "core-032";
const TRUE_STRIKE: &str = "core-044";
const SORCERER: &str = "core-068";
const ADAPTIVE_UI: &str = "core-074";
const TWINSPELL: &str = "core-079";
const MROW: &str = "core-086";
const CORPSE_EATER: &str = "core-089";
const FAUCI: &str = "core-091";
const RENO: &str = "core-053";
const BREAD: &str = "core-018";
const HINDER: &str = "core-021"; // cast on draw; its Radiant face asks nothing (R431)
const HONEYPOT: &str = "core-060";
const BREAD_TOKEN: &str = "core-t-bread";
const RUSH_TOKEN: &str = "core-t-rush";
const LIBRARY: [&str; 8] = [
    VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA,
];

use super::{in_hand, register_fixture_script, scenario};

/// Grant Reborn to a unit on the field directly (R21).
fn grant_reborn(g: &mut Scenario, card: &CardInstance) {
    let live = g.card(&card.id).id.clone();
    find_instance_mut(g.state_mut(), &live)
        .expect("the unit to grant Reborn")
        .granted_keywords
        .push(Keyword::Reborn);
}

fn died(g: &Scenario, card: &CardInstance) -> bool {
    g.events()
        .iter()
        .any(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == card.id))
}

/// The Echo repeat's fresh target prompt, answered with the enemy hero (§10.5 step 6).
fn answer_repeat_with_hero(g: &mut Scenario) {
    let Some(pending) = g.state().pending.clone() else {
        panic!("the Echo repeat should ask for a target");
    };
    let Some(hero) = pending
        .options
        .iter()
        .find(|option| matches!(option.selection, Selection::Hero { player: PlayerId::P2 }))
    else {
        let keys: Vec<&str> = pending.options.iter().map(|option| option.key.as_str()).collect();
        panic!("no enemy hero option: {}", keys.join(", "));
    };
    g.answer(json!([hero.selection]));
}

mod r212_a_reborn_body_does_not_answer_for_the_stay_that_died {
    use super::*;

    #[test]
    fn r212_r174_fed_fauci_that_dies_attacking_and_comes_back_through_reborn_gets_no_plague_counter_for_the_hit_that_killed_it()
     {
        let mut g = scenario(json!({
            "p1": { "hand": [STOCKPILE], "field": [{ "def": FAUCI, "lane": 1, "damage": 5 }], "library": LIBRARY },
            "p2": { "hand": [STOCKPILE], "field": [{ "def": VANILLA, "lane": 1 }], "library": LIBRARY },
        }));
        let fauci = g.unit_at(PlayerId::P1, 1);
        grant_reborn(&mut g, &fauci);

        // A 1/6 with 5 damage attacks a 3/3: the strike back is lethal, and Reborn brings the same
        // instance straight back (§4.5 step 4) before the combat's damage events are dispatched.
        let prey = g.unit_at(PlayerId::P2, 1);
        g.attack(&fauci.id, &prey.id);

        assert!(died(&g, &fauci));
        g.expect_in_zone(&fauci.id, "field");
        assert_eq!(g.card(&fauci.id).reborn_spent, Some(true));
        assert_eq!(g.card(&fauci.id).counters.plague.unwrap_or(0), 0);
    }

    #[test]
    fn r212_r53_r174_fed_fauci_forced_into_moths_to_the_flame_killed_by_the_strike_back_and_reborn_gets_no_plague_counter()
     {
        let mut g = scenario(json!({
            "p1": { "hand": [STOCKPILE], "field": [{ "def": MOTHS, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [STOCKPILE], "field": [{ "def": FAUCI, "lane": 1, "damage": 5 }], "library": LIBRARY },
        }));
        let fauci = g.unit_at(PlayerId::P2, 1);
        grant_reborn(&mut g, &fauci);

        g.start_turn();

        assert!(died(&g, &fauci));
        g.expect_in_zone(&fauci.id, "field");
        assert_eq!(g.card(&fauci.id).counters.plague.unwrap_or(0), 0);
    }

    #[test]
    fn r212_fed_fauci_killed_by_the_first_resolution_of_an_echoed_true_strike_gets_no_plague_counter_on_its_reborn_body_10_5_step_6()
     {
        let mut g = scenario(json!({
            "p1": {
                "hand": [TWINSPELL, TRUE_STRIKE, VANILLA],
                "field": [{ "def": FAUCI, "lane": 1, "damage": 5 }],
                "library": LIBRARY,
            },
            "p2": { "hand": [VANILLA], "library": LIBRARY },
        }));
        let fauci = g.unit_at(PlayerId::P1, 1);
        grant_reborn(&mut g, &fauci);

        g.play(TWINSPELL, json!({}));
        // The check between the resolutions (§4.5) kills Fauci and Reborn returns it; the repeat then
        // asks for a fresh target, and the first hit's damage event is dispatched after all of that.
        g.play(TRUE_STRIKE, json!({ "targets": pick::instance(&fauci) }));
        answer_repeat_with_hero(&mut g);

        assert!(died(&g, &fauci));
        g.expect_in_zone(&fauci.id, "field");
        assert_eq!(g.card(&fauci.id).counters.plague.unwrap_or(0), 0);
    }

    fn trade(reborn: bool) -> Scenario {
        let mut g = scenario(json!({
            "p1": { "hand": [VANILLA], "field": [{ "def": PANTHER, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [VANILLA], "field": [{ "def": SORCERER, "lane": 1 }], "library": LIBRARY },
        }));
        let panther = g.unit_at(PlayerId::P1, 1);
        if reborn {
            grant_reborn(&mut g, &panther);
        }
        let prey = g.unit_at(PlayerId::P2, 1);
        g.attack(&panther.id, &prey.id);
        assert!(died(&g, &panther));
        assert!(died(&g, &prey));
        g
    }

    #[test]
    fn r212_r42_r174_r2396000_a_prem_panther_that_trades_draws_2_once_and_its_reborn_body_draws_nothing_more()
    {
        // Without Reborn the Panther is in its graveyard when its combat closes, and "Whenever this
        // attacks" runs for it all the same, as the snapshot it fought with (R2396000): 2 for the kill.
        let plain = trade(false);
        assert_eq!(plain.hand(PlayerId::P1).len(), 3);
        // With Reborn the same trade happens, and the body back in lane 1 is a new arrival (R83) that
        // attacked nothing: the attack is answered once, by the Panther that fought, and no more.
        let mut back = trade(true);
        let body = back.unit_at(PlayerId::P1, 1);
        back.expect_in_zone(&body.id, "field");
        assert_eq!(back.hand(PlayerId::P1).len(), 3);
    }
}

mod r212_a_card_that_arrived_after_a_death_does_not_answer_it {
    use super::*;

    #[test]
    fn r212_r89_a_corpse_eater_drawn_by_an_echo_repeat_does_not_feed_on_a_unit_that_died_before_it_reached_the_hand_8_89()
     {
        let library = [&[VANILLA, CORPSE_EATER][..], &LIBRARY[..]].concat();
        let mut g = scenario(json!({
            "p1": { "hand": [TWINSPELL, ADAPTIVE_UI], "library": library },
            "p2": { "hand": [VANILLA], "field": [{ "def": GARY, "lane": 1 }], "library": LIBRARY },
        }));
        let gary = g.unit_at(PlayerId::P2, 1);

        g.play(TWINSPELL, json!({}));
        g.play(ADAPTIVE_UI, json!({ "x": 1, "targets": pick::instance(&gary) }));
        answer_repeat_with_hero(&mut g);

        assert!(died(&g, &gary));
        let Some(eater) = g
            .hand(PlayerId::P1)
            .iter()
            .find(|card| card.def_id == CORPSE_EATER)
            .cloned()
        else {
            panic!("Corpse Eater should have been drawn by the repeat");
        };
        assert_eq!(g.card(&eater.id).buffs, AttackHealth { attack: 0, health: 0 });
    }
}

mod r212_a_card_answers_for_the_player_who_controlled_it_when_the_event_happened {
    use super::*;

    #[test]
    fn r212_prem_panther_that_kills_miss_mrow_draws_for_the_player_who_controlled_it_not_for_the_player_mrow_s_death_gave_it_to_8_32_8_conventions()
     {
        let mut g = scenario(json!({
            "p1": { "hand": [RENO], "field": [{ "def": PANTHER, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [RENO], "field": [{ "def": MROW, "lane": 1 }], "library": LIBRARY },
        }));
        let panther = g.unit_at(PlayerId::P1, 1);
        let mrow = g.unit_at(PlayerId::P2, 1);
        let p1_hand = g.hand(PlayerId::P1).len();
        let p2_hand = g.hand(PlayerId::P2).len();

        g.attack(&panther.id, &mrow.id);

        g.expect_in_zone(&mrow.id, "graveyard");
        assert_eq!(g.card(&panther.id).controller, PlayerId::P2);
        assert_eq!(g.hand(PlayerId::P1).len(), p1_hand + 2);
        assert_eq!(g.hand(PlayerId::P2).len(), p2_hand);
    }
}

fn units_of(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
    (1..=5).filter_map(|lane| s.unit(player, lane)).collect()
}

fn fixture_def(id: &str, type_: CardType, cost: i32, face: Value) -> CardDef {
    json_as(json!({
        "id": id,
        "index": id,
        "name": id,
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": face.clone(),
        "radiant": face,
    }))
}

fn costed_fixture(s: &mut Scenario, id: &str, type_: CardType, script: Script, cost: Option<i32>) {
    let face = json!({ "keywords": [], "text": id });
    let def = fixture_def(id, type_, cost.unwrap_or(0), face);
    s.state_mut().transient_defs.insert(id.to_string(), def);
    register_fixture_script(id, script);
}

/// A face-down trap of `player`'s in a backrow lane (R33).
fn set_trap(s: &mut Scenario, def_id: &str, player: PlayerId, lane: i32) -> CardInstance {
    let mut card = new_instance(s.state_mut(), def_id, player, Zone::Hand { player });
    if !place_on_field(
        s.state_mut(),
        &mut card,
        ZoneSlot {
            player,
            row: Row::Backrow,
            lane,
        },
        Default::default(),
    ) {
        panic!("could not place {def_id}");
    }
    let live = find_instance_mut(s.state_mut(), &card.id).expect("the placed trap");
    live.face_up = Some(false);
    live.clone()
}

fn enemy_backrow_lane2(ctx: &EffectContext<'_>) -> Option<CardInstance> {
    card_at(
        &*ctx.state,
        ZoneSlot {
            player: opponent_of(ctx.controller),
            row: Row::Backrow,
            lane: 2,
        },
    )
    .cloned()
}

fn trap_fired_for(events: &[GameEvent], instance_id: &str) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, GameEvent::TrapFired { instance_id: id, .. } if id == instance_id))
        .count()
}

mod r212_for_traps_a_trap_answers_an_event_as_the_board_stood_when_it_happened {
    use super::*;

    #[test]
    fn r212_a_bear_honeypot_its_opponent_s_spell_stole_after_a_cast_in_the_same_list_answers_that_cast_for_the_player_who_held_it_then()
     {
        let mut s = scenario(json!({
            // Arbitrary seed: no pool or random roll decides this test (audited under #848).
            "seed": "edge-r8-honeypot-lifted",
            "p1": {
                "hand": [VANILLA],
                "field": [{ "def": VANILLA, "lane": 5 }],
                "library": [{ "def": HINDER, "radiant": true }, VANILLA, VANILLA, VANILLA],
            },
            "p2": {
                "hand": [VANILLA],
                "field": [{ "def": VANILLA, "lane": 5 }],
                "backrow": [{ "def": HONEYPOT, "lane": 2, "faceUp": false }],
                "library": LIBRARY,
            },
        }));
        let trap = must(s.backrow(PlayerId::P2, 2), "p2's Bear Honeypot");
        // a cast on draw is no play since #2179, R2179002, so no trap answers it.)
        costed_fixture(
            &mut s,
            "edge-r8-draw-then-lift",
            CardType::Spell,
            Script {
                cry: Some(hook(|ctx| {
                    let enemy = enemy_backrow_lane2(ctx);
                    let mut list = vec![effects::cast_new(effects::CastNewArgs {
                        def: effects::CastNewDef::Id(HINDER.to_string()),
                        radiant: Some(true),
                        how: effects::CastHow::default(),
                    })];
                    if let Some(enemy) = enemy {
                        list.push(effects::steal(json_as(json!({ "instanceId": enemy.id }))));
                    }
                    list
                })),
                ..Script::default()
            },
            Some(2),
        );
        let spell = in_hand(&mut s, "edge-r8-draw-then-lift", PlayerId::P1);

        s.play(&spell.id, json!({}));

        // The Spell cast Hinder (R70: a play, paid 0) while the Honeypot was still p2's, and only then did
        // it take the Honeypot.
        let cast: Vec<GameEvent> = s
            .events()
            .iter()
            .filter(|event| matches!(event, GameEvent::CardResolved { def_id, .. } if def_id == HINDER))
            .cloned()
            .collect();
        assert_eq!(cast.len(), 1);
        assert!(
            matches!(
                &cast[0],
                GameEvent::CardResolved {
                    player: PlayerId::P1,
                    cost_paid: 0,
                    ..
                }
            ),
            "{:?} should match {{ player: \"p1\", costPaid: 0 }}",
            cast[0]
        );
        assert!(s.events().iter().any(|event| matches!(
            event,
            GameEvent::ControlChanged { instance_id, controller: PlayerId::P1, .. } if *instance_id == trap.id
        )));
        // R212: the cast is answered as the board stood when it happened. p2's Honeypot saw its
        // opponent play a card costing 1 or less, so it fires, and its tokens are p2's (R52's shape:
        assert_eq!(trap_fired_for(s.events(), &trap.id), 1);
        assert_eq!(
            units_of(&s, PlayerId::P2)
                .iter()
                .filter(|unit| unit.def_id == RUSH_TOKEN)
                .count(),
            2
        );
        assert_eq!(
            units_of(&s, PlayerId::P1)
                .iter()
                .filter(|unit| unit.def_id == RUSH_TOKEN)
                .count(),
            0
        );
    }

    #[test]
    fn r212_a_bread_and_butter_an_earlier_trap_of_the_end_of_turn_window_stole_answers_that_turn_end_for_the_player_who_held_it_when_it_ended()
     {
        let mut s = scenario(json!({
            "seed": "edge-r8-window-lift",
            "p1": { "hand": [VANILLA], "field": [{ "def": VANILLA, "lane": 5 }], "library": LIBRARY },
            "p2": {
                "hand": [VANILLA],
                "field": [{ "def": VANILLA, "lane": 5 }],
                "backrow": [{ "def": BREAD, "lane": 2, "faceUp": false }],
                "library": LIBRARY,
            },
        }));
        let bread = must(s.backrow(PlayerId::P2, 2), "p2's Bread and Butter");
        costed_fixture(
            &mut s,
            "edge-r8-window-lifter",
            CardType::Trap,
            Script {
                triggers: vec![TriggerDef::new(
                    "edge-r8-window-lifter",
                    &[GameEventType::TurnEnded],
                    |ctx, _event| match enemy_backrow_lane2(ctx) {
                        None => vec![],
                        Some(enemy) => vec![effects::steal(json_as(json!({ "instanceId": enemy.id })))],
                    },
                )],
                ..Script::default()
            },
            None,
        );
        set_trap(&mut s, "edge-r8-window-lifter", PlayerId::P1, 1);

        // p1 ends the turn with 4 unspent mana. The window offers the ending player's traps first (R62),
        // so the lifter takes the Bread and Butter before the Bread and Butter is offered the turn end.
        s.end_turn();

        assert_eq!(s.card(&bread.id).controller, PlayerId::P1);
        // R212: the turn end is answered as the board stood when it happened, and the Bread and Butter
        // was p2's then, so its 4/4 is p2's (R52: "the trap's controller").
        assert_eq!(
            units_of(&s, PlayerId::P2)
                .iter()
                .filter(|unit| unit.def_id == BREAD_TOKEN)
                .count(),
            1
        );
        assert_eq!(
            units_of(&s, PlayerId::P1)
                .iter()
                .filter(|unit| unit.def_id == BREAD_TOKEN)
                .count(),
            0
        );
    }

    #[test]
    fn r212_r174_a_bear_honeypot_that_arrived_after_a_cast_in_the_same_list_does_not_answer_that_cast() {
        let mut s = scenario(json!({
            "seed": "edge-r8-honeypot-late",
            "p1": {
                "hand": [VANILLA],
                "field": [{ "def": VANILLA, "lane": 5 }],
                "library": [{ "def": HINDER, "radiant": true }, VANILLA, VANILLA, VANILLA],
            },
            "p2": { "hand": [VANILLA], "field": [{ "def": VANILLA, "lane": 5 }], "library": LIBRARY },
        }));
        costed_fixture(
            &mut s,
            "edge-r8-draw-then-gift-trap",
            CardType::Spell,
            Script {
                cry: Some(hook(|_ctx| {
                    vec![
                        effects::draw(json_as(json!({ "count": 1 }))),
                        effects::summon(json_as(json!({ "defId": HONEYPOT, "player": "enemy" }))),
                    ]
                })),
                ..Script::default()
            },
            Some(2),
        );
        let spell = in_hand(&mut s, "edge-r8-draw-then-gift-trap", PlayerId::P1);

        s.play(&spell.id, json!({}));

        assert_eq!(
            s.events()
                .iter()
                .filter(|event| matches!(event, GameEvent::CardResolved { def_id, .. } if def_id == HINDER))
                .count(),
            1
        );
        let (honeypot_id, honeypot_lane) = must(
            s.events().iter().find_map(|event| match event {
                GameEvent::Summoned {
                    instance_id,
                    def_id,
                    lane,
                    ..
                } if def_id == HONEYPOT => Some((instance_id.clone(), *lane)),
                _ => None,
            }),
            "the Honeypot's summon",
        );
        // The only play costing 1 or less is Hinder's cast, which happened before the Honeypot was on
        // the field: a stay that did not see it, so it does not answer it and stays armed (R212, R99).
        assert_eq!(trap_fired_for(s.events(), &honeypot_id), 0);
        assert_eq!(
            units_of(&s, PlayerId::P2)
                .iter()
                .filter(|unit| unit.def_id == RUSH_TOKEN)
                .count(),
            0
        );
        assert_eq!(
            s.backrow(PlayerId::P2, honeypot_lane).map(|card| card.id.clone()),
            Some(honeypot_id)
        );
    }
}

// A response meets the stay its event's card had when the event happened (R174, R212)

const RIGHT_HOUSE: &str = "core-003";

#[derive(Default)]
struct UnitOpts {
    attack: Option<i32>,
    health: Option<i32>,
    keywords: Option<Vec<Keyword>>,
    cost: Option<i32>,
}

fn unit_fixture(s: &mut Scenario, id: &str, script: Script, opts: UnitOpts) {
    let face = json!({
        "attack": opts.attack.unwrap_or(2),
        "health": opts.health.unwrap_or(2),
        "keywords": opts.keywords.unwrap_or_default(),
        "text": id,
    });
    let def = fixture_def(id, CardType::Unit, opts.cost.unwrap_or(0), face);
    s.state_mut().transient_defs.insert(id.to_string(), def);
    register_fixture_script(id, script);
}

fn unit_on_field(s: &mut Scenario, def_id: &str, player: PlayerId, lane: i32) -> CardInstance {
    let mut card = new_instance(s.state_mut(), def_id, player, Zone::Hand { player });
    if !place_on_field(
        s.state_mut(),
        &mut card,
        ZoneSlot {
            player,
            row: Row::Units,
            lane,
        },
        Default::default(),
    ) {
        panic!("could not place {def_id}");
    }
    find_instance(s.state(), &card.id)
        .cloned()
        .expect("the placed unit")
}

fn find_index(events: &[GameEvent], pick: impl Fn(&GameEvent) -> bool) -> i64 {
    events.iter().position(pick).map(|at| at as i64).unwrap_or(-1)
}

mod r174_r212_a_late_dispatched_card_resolved_meets_the_played_card_s_stay {
    use super::*;

    #[test]
    fn r174_r61_r70_r83_bear_honeypot_s_tokens_do_not_attack_the_reborn_body_of_a_cast_unit_that_died_before_its_card_resolved_was_dispatched()
     {
        let mut s = scenario(json!({
            "p1": { "hand": [RENO], "library": [RENO, RENO, RENO] },
            "p2": { "backrow": [{ "def": HONEYPOT, "lane": 1 }], "hand": [RENO], "library": [RENO, RENO] },
        }));
        // A Unit with Reborn, and a 2-cost Spell whose list casts one (E12, R453) and then destroys every
        // unit. (It drew a cast-on-draw one through v0.5.2; a cast on draw is no play since #2179,
        // R2179002, so no trap answers it.)
        unit_fixture(
            &mut s,
            "edge-r10-cast-reborn-unit",
            Script::default(),
            UnitOpts {
                keywords: Some(vec![Keyword::Reborn]),
                cost: Some(1),
                ..Default::default()
            },
        );
        costed_fixture(
            &mut s,
            "edge-r10-draw-then-sweep",
            CardType::Spell,
            Script {
                cry: Some(hook(|_ctx| {
                    vec![
                        effects::cast_new(effects::CastNewArgs {
                            def: effects::CastNewDef::Id("edge-r10-cast-reborn-unit".to_string()),
                            radiant: None,
                            how: effects::CastHow::default(),
                        }),
                        effects::destroy_all(json_as(json!({ "side": "any" }))),
                    ]
                })),
                ..Script::default()
            },
            Some(2),
        );
        let spell = in_hand(&mut s, "edge-r10-draw-then-sweep", PlayerId::P1);

        s.play(&spell.id, json!({}));

        // The Spell cast the Unit (a play, R70, cost paid 0), whose step 7 emitted `cardResolved` with the
        // Unit in play; then the Spell's own sweep destroyed it, and Reborn brought a new body back (§4.5
        // step 4, R83) before the Spell's loop dispatched that `cardResolved` to the traps.
        let cast = must(
            s.events().iter().find_map(|event| match event {
                GameEvent::CardResolved {
                    instance_id, def_id, ..
                } if def_id == "edge-r10-cast-reborn-unit" => find_instance(s.state(), instance_id).cloned(),
                _ => None,
            }),
            "the cast Unit",
        );
        let resolved_at = find_index(
            s.events(),
            |event| matches!(event, GameEvent::CardResolved { instance_id, .. } if *instance_id == cast.id),
        );
        let died_at = find_index(
            s.events(),
            |event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == cast.id),
        );
        assert!(resolved_at >= 0);
        assert!(died_at > resolved_at);
        assert!(
            s.events()
                .iter()
                .any(|event| matches!(event, GameEvent::TrapFired { .. }))
        );
        assert!(
            !s.events().iter().any(|event| matches!(
                event,
                GameEvent::AttackDeclared { target_id, .. } if *target_id == cast.id
            )),
            "no forced attack on the Reborn body"
        );
        s.expect_in_zone(&cast.id, "field");
    }
}

fn opponents_play(controller: PlayerId, event: &GameEvent) -> Option<String> {
    match event {
        GameEvent::CardPlayed {
            player, instance_id, ..
        } if *player != controller => Some(instance_id.clone()),
        _ => None,
    }
}

mod r174_a_queued_trigger_aimed_at_the_card_its_event_names_meets_that_card_s_stay {
    use super::*;

    #[test]
    fn r174_r83_r59_a_trigger_naming_the_played_unit_by_its_event_s_id_does_not_buff_its_reborn_body_after_an_earlier_trigger_on_the_same_play_killed_it()
     {
        let mut s = scenario(json!({
            "p1": { "hand": [RIGHT_HOUSE, RENO], "library": [RENO, RENO] },
            "p2": { "hand": [RENO], "library": [RENO] },
        }));
        // Two of p2's units answer p1's plays, in lane order (R68): the first destroys the played unit,
        // the second gives it +5 attack.
        unit_fixture(
            &mut s,
            "edge-r10-slayer",
            Script {
                triggers: vec![TriggerDef::new(
                    "slay",
                    &[GameEventType::CardPlayed],
                    |ctx, event| match opponents_play(ctx.controller, event) {
                        None => vec![],
                        Some(id) => vec![effects::destroy(json_as(json!({
                            "target": { "of": "instance", "instanceId": id },
                        })))],
                    },
                )],
                ..Script::default()
            },
            UnitOpts::default(),
        );
        unit_fixture(
            &mut s,
            "edge-r10-marker",
            Script {
                triggers: vec![TriggerDef::new(
                    "mark",
                    &[GameEventType::CardPlayed],
                    |ctx, event| match opponents_play(ctx.controller, event) {
                        None => vec![],
                        Some(id) => vec![effects::buff(json_as(json!({
                            "target": { "of": "instance", "instanceId": id },
                            "attack": 5,
                        })))],
                    },
                )],
                ..Script::default()
            },
            UnitOpts::default(),
        );
        unit_on_field(&mut s, "edge-r10-slayer", PlayerId::P2, 1);
        unit_on_field(&mut s, "edge-r10-marker", PlayerId::P2, 2);
        let played = s.card(RIGHT_HOUSE).clone();

        s.play(&played.id, json!({}));

        // The first trigger destroyed it, the check after that trigger collected it (R59), and Reborn
        // brought a new body back into its zone (§4.5 step 4): a new arrival, which nobody played (R83).
        assert!(s.events().iter().any(
            |event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == played.id)
        ));
        s.expect_in_zone(&played.id, "field");
        assert_eq!(s.card(&played.id).reborn_spent, Some(true));
        assert!(!s.events().iter().any(
            |event| matches!(event, GameEvent::Buffed { instance_id, .. } if *instance_id == played.id)
        ));
        assert_eq!(s.stats(&played.id).attack, 1);
    }
}

mod r174_only_the_card_a_queued_trigger_s_event_names_is_judged_from_when_the_event_happened {
    use super::*;

    // R174's row takes the event's stay for the card the event names alone: a card a queued trigger
    // reads off the board as it resolves, such as a Reborn body an earlier trigger on the same event
    // made, is on the stay it has then, so a buff by id reaches it as the same buff over the board does.
    const HIT_JOB: &str = "core-016";

    fn enemy_died(controller: PlayerId, event: &GameEvent) -> bool {
        matches!(event, GameEvent::Destroyed { owner, .. } if *owner != controller)
    }

    #[derive(Clone, Copy)]
    enum PerUnit {
        ById,
        OverTheBoard,
    }

    impl PerUnit {
        fn name(self) -> &'static str {
            match self {
                PerUnit::ById => "byId",
                PerUnit::OverTheBoard => "overTheBoard",
            }
        }
    }

    struct Board {
        reborn: CardInstance,
        victim: CardInstance,
    }

    fn board(s: &mut Scenario, per_unit: PerUnit) -> Board {
        unit_fixture(
            s,
            "edge-r11-reborn",
            Script::default(),
            UnitOpts {
                attack: Some(1),
                health: Some(1),
                keywords: Some(vec![Keyword::Reborn]),
                ..Default::default()
            },
        );
        let reborn = unit_on_field(s, "edge-r11-reborn", PlayerId::P1, 3);
        let reborn_id = reborn.id.clone();
        unit_fixture(
            s,
            "edge-r11-striker",
            Script {
                triggers: vec![TriggerDef::new(
                    "strike",
                    &[GameEventType::Destroyed],
                    move |ctx, event| {
                        if enemy_died(ctx.controller, event) {
                            vec![effects::damage(json_as(json!({
                                "to": { "of": "instance", "instanceId": reborn_id },
                                "amount": 5,
                            })))]
                        } else {
                            vec![]
                        }
                    },
                )],
                ..Script::default()
            },
            UnitOpts {
                attack: Some(1),
                health: Some(5),
                ..Default::default()
            },
        );
        let rally_id = format!("edge-r11-rally-{}", per_unit.name());
        unit_fixture(
            s,
            &rally_id,
            Script {
                triggers: vec![TriggerDef::new(
                    "rally",
                    &[GameEventType::Destroyed],
                    move |ctx, event| {
                        if !enemy_died(ctx.controller, event) {
                            return vec![];
                        }
                        match per_unit {
                            PerUnit::OverTheBoard => {
                                vec![effects::buff_all_units(json_as(
                                    json!({ "side": "self", "attack": 1 }),
                                ))]
                            }
                            PerUnit::ById => vec![effects::for_each_card(effects::ForEachCardArgs {
                                cards: Arc::new(|c: &mut EffectContext<'_>| {
                                    active_units_of(&*c.state, c.controller)
                                        .iter()
                                        .map(|card| card.id.clone())
                                        .collect::<Vec<String>>()
                                }),
                                each: Arc::new(|id: &str| {
                                    effects::buff(json_as(json!({
                                        "target": { "of": "instance", "instanceId": id },
                                        "attack": 1,
                                    })))
                                }),
                            })],
                        }
                    },
                )],
                ..Script::default()
            },
            UnitOpts {
                attack: Some(1),
                health: Some(5),
                ..Default::default()
            },
        );
        unit_on_field(s, "edge-r11-striker", PlayerId::P1, 1);
        unit_on_field(s, &rally_id, PlayerId::P1, 2);
        let victim = must(s.unit(PlayerId::P2, 1), "p2's unit");
        Board { reborn, victim }
    }

    fn reaches_the_reborn_body(per_unit: PerUnit) {
        let mut s = scenario(json!({
            "p1": { "hand": [HIT_JOB], "mana": 4, "library": LIBRARY },
            "p2": { "field": [VANILLA], "library": LIBRARY },
        }));
        let Board { reborn, victim } = board(&mut s, per_unit);

        s.play(HIT_JOB, json!({ "targets": pick::instance(&victim) }));

        // The striker answered the death first (lane 1, R68) and killed the 1/1; the check after it
        // brought a Reborn body back (§4.5 step 4, R59). The rally, queued on the same death, reads
        // p1's units as it resolves, the body among them, and each one is aimed at the stay it stands
        // on now: the event named none of them.
        assert!(died(&s, &reborn));
        s.expect_in_zone(&reborn.id, "field");
        assert_eq!(
            s.events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Buffed { instance_id, .. } if *instance_id == reborn.id))
                .count(),
            1
        );
        assert_eq!(s.stats(&reborn.id).attack, 2);
    }

    #[test]
    fn r174_r59_r83_a_trigger_that_buffs_each_of_its_side_s_units_over_the_board_reaches_the_reborn_body_an_earlier_trigger_on_the_same_death_made()
     {
        reaches_the_reborn_body(PerUnit::OverTheBoard);
    }

    #[test]
    fn r174_r59_r83_a_trigger_that_buffs_each_of_its_side_s_units_by_id_reaches_the_reborn_body_an_earlier_trigger_on_the_same_death_made()
     {
        reaches_the_reborn_body(PerUnit::ById);
    }
}
