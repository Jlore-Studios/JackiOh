//! Port of `packages/cards/test/control-change.test.ts` (part 27.1).
//!
//! R171 and R172 with the real cards (SPEC §4.1, §11; docs/polish/4-edge-cases.md behaviours 1 to 10,
//! 14 and 15). Every card that changes control — #36 radiant, #49, #50, #52, #86 and #87 — makes the
//! unit it moves enter its new controller's side on that turn: summoning sick exactly as a unit
//! summoned that turn is, with Rush and Charge applying as usual, and with a fresh exertion for its
//! new controller. A unit that only moves between lanes on its own side has entered nothing.
//!
//! R172 is the case the brief asked to check: a stolen unit dies as its controller's. Its Death runs
//! for that player (#81, radiant #3) and a Reborn body comes back on that player's side, sick.
//!
//! Two cases the hunt's second round added at the end, where a card crossing the centre line meets
//! another rule: radiant #52 still takes the opponent's card that crosses onto its side (R14), and a
//! played #52 that crossed onto #85's side is not a Fuse target for itself (R61).
//!
//! The engine-fixture proofs of the same rows are `packages/engine/test/control-change.test.ts` and
//! `control-change.property.test.ts`; this file is the proof through `scenario()` that the cards
//! reach the rule.
//!
//! Every case that crosses a turn boundary keeps a card in hand or a unit on the board for both
//! sides, so R82's automatic turn end never skips a turn (see the harness header).
//!
//! Props: #25 4-mana 7/7 (no Rush or Charge; Armor 7, so a small attacker bounces off it), #8 Mr.
//! Vanilla (a plain 4/4), #20 Pointmaster (a 7/1 that never matters), #56 Jilliax (Rush), #45 Deft Duelist (Charge), #11 Tempo Timmy (Charge radiant), #14 Jlockeed's Weapons
//! (an aura granting Rush), #68 Twisted Sorcerer (4 damage to a target), #41 Sheepish (a Trap).

use jackioh_engine::testkit::*;

const MAGIC_JAMMED: &str = "core-036";
const MIND_CONTROL: &str = "core-049";
const KPOP: &str = "core-050";
const SILAS: &str = "core-052";
const MROW: &str = "core-086";
const CHAOS: &str = "core-087";
const SAINTESS: &str = "core-081";
const RIGHT_HOUSE: &str = "core-003";
const SEVEN_SEVEN: &str = "core-025";
const VANILLA: &str = "core-008";
const POINTMASTER: &str = "core-020";
const JILLIAX: &str = "core-056";
const DUELIST: &str = "core-045";
const TIMMY: &str = "core-011";
const WEAPONS: &str = "core-014";
const SORCERER: &str = "core-068";
const SHEEPISH: &str = "core-041";
const UNLICENSED: &str = "core-085";
const LIBRARY: [&str; 4] = [VANILLA, VANILLA, VANILLA, VANILLA];

// TS's `/summoning sick/`, `/Rush cannot hit the hero/`, `/already acted/`: literal patterns.
const SICK: &str = "summoning sick";
const RUSH_NOT_HERO: &str = "Rush cannot hit the hero";
const ALREADY_ACTED: &str = "already acted";

/// The harness with the real catalog and every card script registered (TS's `_harness.ts` import
/// ran `registerAll()`; the engine's testkit cannot name the cards crate, so the cards test does).
fn setup(opts: Value) -> Scenario {
    jackioh_cards::register_all();
    scenario(opts)
}

fn unit_at(g: &Scenario, player: &'static str, lane: i32) -> CardInstance {
    g.unit(player, lane)
        .unwrap_or_else(|| panic!("setup: {player} should hold a unit in lane {lane}"))
}

fn backrow_at(g: &Scenario, player: &'static str, lane: i32) -> CardInstance {
    g.backrow(player, lane)
        .unwrap_or_else(|| panic!("setup: {player} should hold a backrow card in lane {lane}"))
}

fn at(card: &CardInstance) -> Value {
    json!([{ "pick": "instance", "instanceId": card.id }])
}

/// The attacks `legalActions` offers this unit right now, as target ids.
fn offered_attacks(g: &Scenario, card: &CardInstance) -> Vec<String> {
    let live = g.card(card).clone();
    legal_actions(g.state(), live.controller)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::Attack { attacker_id, target_id, .. } if attacker_id == live.id => Some(target_id),
            _ => None,
        })
        .collect()
}

fn offers_switch(g: &Scenario, card: &CardInstance) -> bool {
    let live = g.card(card).clone();
    legal_actions(g.state(), live.controller)
        .iter()
        .any(|action| matches!(action, ActionBody::SwitchPosition { instance_id, .. } if *instance_id == live.id))
}

/// R171's bookkeeping: entered on this turn, with a fresh exertion.
fn expect_entered_now(g: &Scenario, card: &CardInstance) {
    let live = g.card(card).clone();
    assert_eq!(
        live.summoned_turn,
        Some(g.state().turn),
        "{} ({}) summonedTurn",
        live.id,
        live.def_id
    );
    assert_eq!(
        serde_json::to_value(live.exertion).unwrap(),
        json!({ "attacked": false, "switched": false }),
        "{} ({}) exertion",
        live.id,
        live.def_id
    );
}

fn hero_health(g: &Scenario, player: PlayerId) -> i32 {
    g.state().players[player].hero.health
}

fn keyword_kinds(g: &Scenario, card: &CardInstance) -> Vec<String> {
    g.stats(card)
        .keywords
        .iter()
        .map(|k| serde_json::to_value(k).unwrap()["kind"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// R171 with the cards that change control
mod r171_with_the_cards_that_change_control {
    use super::*;

    #[test]
    fn r171_c49_a_stolen_unit_with_neither_rush_nor_charge_cannot_attack_that_turn_and_attacks_on_the_thiefs_next_turn() {
        let mut g = setup(json!({
            "p1": { "hand": [MIND_CONTROL, VANILLA], "library": LIBRARY },
            "p2": {
                "hand": [VANILLA],
                "field": [{ "def": SEVEN_SEVEN, "lane": 3 }, { "def": VANILLA, "lane": 5 }],
                "library": LIBRARY,
            },
        }));
        let prey = unit_at(&g, "p2", 3);
        let bystander = unit_at(&g, "p2", 5);

        g.play(MIND_CONTROL, json!({ "targets": at(&prey) }));

        assert_eq!(g.card(&prey).controller, PlayerId::P1);
        expect_entered_now(&g, &prey);
        g.expect_refused_with(|g| g.attack(&prey, "hero"), SICK);
        g.expect_refused_with(|g| g.attack(&prey, &bystander), SICK);
        assert_eq!(offered_attacks(&g, &prey), Vec::<String>::new());

        g.end_turn().end_turn();
        assert_eq!(g.state().active, PlayerId::P1);
        assert!(offered_attacks(&g, &prey).contains(&"hero-p2".to_string()));
        let before = hero_health(&g, PlayerId::P2);
        g.attack(&prey, "hero");
        g.expect_health("p2", before - 7);
    }

    #[test]
    fn r171_c36_radiant_a_stolen_backrow_card_takes_the_turn_and_a_fresh_exertion_like_any_other_card() {
        let mut g = setup(json!({
            "p1": { "hand": [{ "def": MAGIC_JAMMED, "radiant": true }], "field": [{ "def": VANILLA, "lane": 1 }] },
            "p2": { "backrow": [{ "def": SHEEPISH, "lane": 3 }], "field": [{ "def": VANILLA, "lane": 1 }] },
        }));
        let trap = backrow_at(&g, "p2", 3);

        g.play(MAGIC_JAMMED, json!({ "targets": at(&trap) }));

        // The Lock lands on p2's lane 3, the original zone, so R15 puts the trap in p1's lane 3.
        assert_eq!(g.backrow("p1", 3).map(|c| c.id), Some(trap.id.clone()));
        assert_eq!(g.card(&trap).controller, PlayerId::P1);
        expect_entered_now(&g, &trap);
    }

    #[test]
    fn r171_c56_base_a_stolen_rush_unit_may_attack_an_enemy_unit_that_turn_but_not_the_hero() {
        let mut g = setup(json!({
            "p1": { "hand": [MIND_CONTROL, VANILLA] },
            "p2": { "field": [{ "def": JILLIAX, "lane": 2 }, { "def": VANILLA, "lane": 4, "damage": 1 }] },
        }));
        let jilliax = unit_at(&g, "p2", 2);
        let vanilla = unit_at(&g, "p2", 4);

        g.play(MIND_CONTROL, json!({ "targets": at(&jilliax) }));

        expect_entered_now(&g, &jilliax);
        g.expect_refused_with(|g| g.attack(&jilliax, "hero"), RUSH_NOT_HERO);
        assert_eq!(offered_attacks(&g, &jilliax), std::slice::from_ref(&vanilla.id));
        g.attack(&jilliax, &vanilla);
        g.expect_in_zone(&vanilla, "graveyard");
    }

    #[test]
    fn r171_c45_a_stolen_charge_unit_that_attacked_for_its_owner_the_turn_before_may_attack_the_hero_at_once() {
        let mut g = setup(json!({
            "active": "p2",
            "turn": 8,
            "p1": { "hand": [MIND_CONTROL, VANILLA], "field": [{ "def": VANILLA, "lane": 5 }], "library": LIBRARY },
            "p2": { "hand": [VANILLA], "field": [{ "def": DUELIST, "lane": 1 }], "library": LIBRARY },
        }));
        let duelist = unit_at(&g, "p2", 1);
        g.attack(&duelist, "hero");
        g.end_turn();
        assert_eq!(g.state().active, PlayerId::P1);
        // Exertion resets at its controller's own turn start, so it is still spent on p1's turn.
        assert!(g.card(&duelist).exertion.attacked);

        g.play(MIND_CONTROL, json!({ "targets": at(&duelist) }));

        expect_entered_now(&g, &duelist);
        assert!(offered_attacks(&g, &duelist).contains(&"hero-p2".to_string()));
        let before = hero_health(&g, PlayerId::P2);
        g.attack(&duelist, "hero");
        g.expect_health("p2", before - 4);
    }

    #[test]
    fn r171_c49_a_stolen_unit_that_switched_position_for_its_owner_may_switch_again_on_the_turn_it_is_stolen() {
        let mut g = setup(json!({
            "active": "p2",
            "turn": 8,
            "p1": { "hand": [MIND_CONTROL, VANILLA], "field": [{ "def": POINTMASTER, "lane": 5 }], "library": LIBRARY },
            "p2": { "hand": [VANILLA], "field": [{ "def": VANILLA, "lane": 3 }, { "def": POINTMASTER, "lane": 5 }], "library": LIBRARY },
        }));
        let switcher = unit_at(&g, "p2", 3);
        g.switch_position(&switcher);
        assert_eq!(g.card(&switcher).position, Some(Position::Def));
        g.end_turn();
        assert!(g.card(&switcher).exertion.switched);

        g.play(MIND_CONTROL, json!({ "targets": at(&switcher) }));

        expect_entered_now(&g, &switcher);
        assert!(offers_switch(&g, &switcher));
        g.switch_position(&switcher);
        assert_eq!(g.card(&switcher).position, Some(Position::Atk));
        // Still sick: the switch was its exertion, and it could not have attacked anyway.
        g.expect_refused_with(|g| g.attack(&switcher, "hero"), ALREADY_ACTED);
    }

    #[test]
    fn r171_c50_k_pop_fanatics_delayed_steal_leaves_the_unit_sick_for_that_whole_turn_and_it_attacks_on_the_next() {
        let mut g = setup(json!({
            "p1": { "hand": [KPOP, VANILLA], "library": LIBRARY },
            "p2": {
                "hand": [VANILLA],
                "field": [{ "def": SEVEN_SEVEN, "lane": 2 }, { "def": VANILLA, "lane": 5 }],
                "library": LIBRARY,
            },
        }));
        let prey = unit_at(&g, "p2", 2);
        g.play(KPOP, json!({ "targets": at(&prey) }));

        g.end_turn().end_turn();

        // The steal ran at p1's start of turn, after the exertion reset: the headline bug.
        assert_eq!(g.state().active, PlayerId::P1);
        assert_eq!(g.card(&prey).controller, PlayerId::P1);
        expect_entered_now(&g, &prey);
        g.expect_refused_with(|g| g.attack(&prey, "hero"), SICK);
        assert_eq!(offered_attacks(&g, &prey), Vec::<String>::new());

        g.end_turn().end_turn();

        assert_eq!(g.state().active, PlayerId::P1);
        let before = hero_health(&g, PlayerId::P2);
        g.attack(&prey, "hero");
        g.expect_health("p2", before - 7);
    }

    #[test]
    fn r171_r361_c86_mrow_dying_on_its_controllers_own_turn_takes_a_killer_that_is_sick_that_turn() {
        let mut g = setup(json!({
            "p1": { "hand": [VANILLA], "field": [{ "def": MROW, "radiant": true, "lane": 1 }] },
            "p2": { "hand": [VANILLA], "field": [{ "def": SEVEN_SEVEN, "lane": 3 }, { "def": VANILLA, "lane": 4 }] },
        }));
        let mrow = unit_at(&g, "p1", 1);
        let seven_seven = unit_at(&g, "p2", 3);
        let vanilla = unit_at(&g, "p2", 4);

        // Radiant Mrow has Rush: 2 into Armor 7 is nothing, and 7 back kills it, so the 7/7 destroyed it.
        g.attack(&mrow, &seven_seven);

        g.expect_in_zone(&mrow, "graveyard");
        assert_eq!(g.card(&seven_seven).controller, PlayerId::P1);
        expect_entered_now(&g, &seven_seven);
        g.expect_refused_with(|g| g.attack(&seven_seven, "hero"), SICK);
        assert_eq!(offered_attacks(&g, &seven_seven), Vec::<String>::new());
        // The unit that had no part in it stays with p2.
        assert_eq!(g.card(&vanilla).controller, PlayerId::P2);
    }

    #[test]
    fn r171_r361_c86_mrow_dying_on_the_opponents_turn_takes_a_killer_that_attacks_freely_on_the_thiefs_next_turn() {
        let mut g = setup(json!({
            "active": "p2",
            "turn": 8,
            "p1": { "hand": [VANILLA], "field": [{ "def": MROW, "lane": 1 }], "library": LIBRARY },
            "p2": {
                "hand": [VANILLA],
                "field": [{ "def": SEVEN_SEVEN, "lane": 3 }, { "def": VANILLA, "lane": 4 }],
                "library": LIBRARY,
            },
        }));
        let mrow = unit_at(&g, "p1", 1);
        let seven_seven = unit_at(&g, "p2", 3);
        let vanilla = unit_at(&g, "p2", 4);

        g.attack(&seven_seven, &mrow);

        g.expect_in_zone(&mrow, "graveyard");
        assert_eq!(g.card(&seven_seven).controller, PlayerId::P1);
        assert_eq!(g.card(&vanilla).controller, PlayerId::P2);
        // The attacker's spent exertion stayed with p2: for p1 it is fresh (R171).
        expect_entered_now(&g, &seven_seven);

        g.end_turn();
        assert_eq!(g.state().active, PlayerId::P1);
        let before = hero_health(&g, PlayerId::P2);
        // p2's Mr. Vanilla walls nothing (no Taunt), so the 7/7 goes to the hero.
        g.attack(&seven_seven, "hero");
        g.expect_health("p2", before - 7);
    }

    #[test]
    fn r171_c87_every_card_the_board_swap_moves_enters_its_new_side_the_casters_are_sick_the_opponents_are_not() {
        let mut g = setup(json!({
            "p1": { "hand": [CHAOS, VANILLA], "field": [{ "def": VANILLA, "lane": 1 }], "library": LIBRARY },
            "p2": {
                "hand": [VANILLA],
                "field": [{ "def": DUELIST, "lane": 2 }, { "def": SEVEN_SEVEN, "lane": 3 }],
                "backrow": [{ "def": SHEEPISH, "lane": 4 }],
                "library": LIBRARY,
            },
        }));
        let mine = unit_at(&g, "p1", 1);
        let duelist = unit_at(&g, "p2", 2);
        let seven_seven = unit_at(&g, "p2", 3);
        let trap = backrow_at(&g, "p2", 4);

        g.play(CHAOS, json!({ "modes": ["board"] }));

        for card in [&mine, &duelist, &seven_seven, &trap] {
            expect_entered_now(&g, card);
        }
        assert_eq!(g.card(&trap).controller, PlayerId::P1);
        // The caster's new units: Charge still charges, anything else waits.
        g.expect_refused_with(|g| g.attack(&seven_seven, "hero"), SICK);
        assert_eq!(offered_attacks(&g, &seven_seven), Vec::<String>::new());
        let before = hero_health(&g, PlayerId::P2);
        g.attack(&duelist, "hero");
        g.expect_health("p2", before - 4);

        // The opponent's new unit entered on p1's turn, so it is ready on p2's.
        g.end_turn();
        assert_eq!(g.state().active, PlayerId::P2);
        assert_eq!(g.card(&mine).controller, PlayerId::P2);
        let mine_before = hero_health(&g, PlayerId::P1);
        g.attack(&mine, "hero");
        g.expect_health("p1", mine_before - 4);
    }

    #[test]
    fn r171_c52_cards_that_cross_the_centre_line_enter_their_new_side_cards_moving_along_their_own_side_keep_their_readiness()
     {
        let mut g = setup(json!({
            "p1": {
                "hand": [SILAS, VANILLA],
                "field": [
                    { "def": VANILLA, "lane": 2 },
                    { "def": POINTMASTER, "lane": 3 },
                    { "def": VANILLA, "lane": 5 },
                ],
            },
            "p2": { "field": [{ "def": SEVEN_SEVEN, "lane": 1 }, { "def": VANILLA, "lane": 4 }] },
        }));
        let spent = unit_at(&g, "p1", 2);
        let ready = unit_at(&g, "p1", 3);
        let outbound = unit_at(&g, "p1", 5);
        let inbound = unit_at(&g, "p2", 1);
        g.attack(&spent, "hero");

        // Right from p1's seat: p1 lane n → n+1, p1 lane 5 → p2 lane 5, p2 lane 1 → p1 lane 1.
        g.play(SILAS, json!({ "zone": 1, "modes": ["right"] }));

        assert_eq!(g.card(&inbound).controller, PlayerId::P1);
        assert_eq!(g.card(&outbound).controller, PlayerId::P2);
        expect_entered_now(&g, &inbound);
        expect_entered_now(&g, &outbound);
        g.expect_refused_with(|g| g.attack(&inbound, "hero"), SICK);

        // Along its own side: the spent unit is still spent, the ready one still ready.
        assert_eq!(g.card(&spent).summoned_turn, None);
        g.expect_refused_with(|g| g.attack(&spent, "hero"), ALREADY_ACTED);
        assert_eq!(g.card(&ready).summoned_turn, None);
        let before = hero_health(&g, PlayerId::P2);
        g.attack(&ready, "hero");
        g.expect_health("p2", before - 7);
    }

    #[test]
    fn r171_c52_radiant_only_the_card_crossing_onto_its_side_is_marked_the_one_it_would_lose_goes_home() {
        let mut g = setup(json!({
            "p1": {
                "hand": [{ "def": SILAS, "radiant": true }, VANILLA],
                "field": [{ "def": POINTMASTER, "lane": 3 }, { "def": VANILLA, "lane": 5 }],
            },
            "p2": { "field": [{ "def": SEVEN_SEVEN, "lane": 1 }, { "def": VANILLA, "lane": 4 }] },
        }));
        let ready = unit_at(&g, "p1", 3);
        let outbound = unit_at(&g, "p1", 5);
        let inbound = unit_at(&g, "p2", 1);

        g.play(SILAS, json!({ "zone": 1, "modes": ["right"] }));

        // "Cards that would move to the opponent are bounced": p1's lane-5 Vanilla goes home at 0 and
        // never changes sides, so it is not marked (R14).
        g.expect_in_zone(&outbound, "hand");
        // p2's lane-1 7/7 moves to p1, which the radiant cell does not touch: it crosses, changes
        // control, and has entered p1's side this turn (R14, R171).
        let changed: Vec<String> = g
            .last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::ControlChanged { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(changed, std::slice::from_ref(&inbound.id));
        assert_eq!(g.card(&inbound).summoned_turn, Some(g.state().turn));
        assert_eq!(offered_attacks(&g, &inbound), Vec::<String>::new());
        // A card moving along its own side is not marked at all.
        assert_eq!(g.card(&ready).summoned_turn, None);
        assert!(offered_attacks(&g, &ready).contains(&"hero-p2".to_string()));
    }

    #[test]
    fn r171_c14_a_stolen_unit_gets_rush_from_the_thiefs_aura_at_once_which_lets_it_attack_units_only() {
        let mut g = setup(json!({
            "p1": { "hand": [MIND_CONTROL, VANILLA], "backrow": [{ "def": WEAPONS, "lane": 1 }] },
            "p2": { "field": [{ "def": SEVEN_SEVEN, "lane": 3 }, { "def": VANILLA, "lane": 5 }] },
        }));
        let prey = unit_at(&g, "p2", 3);
        let vanilla = unit_at(&g, "p2", 5);

        g.play(MIND_CONTROL, json!({ "targets": at(&prey) }));

        assert!(keyword_kinds(&g, &prey).contains(&"Rush".to_string()));
        g.expect_refused_with(|g| g.attack(&prey, "hero"), RUSH_NOT_HERO);
        assert_eq!(offered_attacks(&g, &prey), std::slice::from_ref(&vanilla.id));
        g.attack(&prey, &vanilla);
        g.expect_in_zone(&vanilla, "graveyard");
    }

    #[test]
    fn r171_c14_rush_from_the_aura_lapses_when_the_aura_leaves_so_a_stolen_unit_and_one_played_this_turn_are_sick_again() {
        let mut g = setup(json!({
            "p1": { "hand": [VANILLA, MIND_CONTROL, MAGIC_JAMMED], "mana": 10, "backrow": [{ "def": WEAPONS, "lane": 1 }] },
            "p2": { "field": [{ "def": SEVEN_SEVEN, "lane": 3 }, { "def": VANILLA, "lane": 5 }] },
        }));
        let weapons = backrow_at(&g, "p1", 1);
        let prey = unit_at(&g, "p2", 3);
        let target = unit_at(&g, "p2", 5);

        g.play(VANILLA, json!({}));
        let fresh = unit_at(&g, "p1", 1);
        g.play(MIND_CONTROL, json!({ "targets": at(&prey) }));

        // Both entered p1's side this turn, and the aura's Rush lets both attack a unit.
        for unit in [&fresh, &prey] {
            expect_entered_now(&g, unit);
            assert!(keyword_kinds(&g, unit).contains(&"Rush".to_string()));
            assert_eq!(offered_attacks(&g, unit), std::slice::from_ref(&target.id));
        }

        // #36 Magic Jammed on p1's own Field Spell: the aura, and the Rush it granted, are gone.
        g.play(MAGIC_JAMMED, json!({ "targets": at(&weapons) }));
        g.expect_in_zone(&weapons, "graveyard");

        for unit in [&fresh, &prey] {
            assert!(!keyword_kinds(&g, unit).contains(&"Rush".to_string()));
            assert_eq!(offered_attacks(&g, unit), Vec::<String>::new());
            g.expect_refused_with(|g| g.attack(unit, &target), SICK);
        }
    }

    #[test]
    fn r171_c49_radiant_making_a_stolen_c11_radiant_gives_it_charge_so_it_may_attack_the_hero_at_once() {
        let mut g = setup(json!({
            // #49 costs (4) since patch v0.2.0 (issue #40): a fifth mana keeps Mr. Vanilla playable, so the
            // turn does not auto-end (R82) and hand p2 a fatigue draw before the hero is read.
            "p1": { "hand": [{ "def": MIND_CONTROL, "radiant": true }, VANILLA], "mana": 5 },
            "p2": { "field": [{ "def": TIMMY, "lane": 2 }, { "def": VANILLA, "lane": 4 }] },
        }));
        let timmy = unit_at(&g, "p2", 2);

        g.play(MIND_CONTROL, json!({ "targets": at(&timmy) }));

        assert!(g.card(&timmy).radiant);
        expect_entered_now(&g, &timmy);
        assert!(offered_attacks(&g, &timmy).contains(&"hero-p2".to_string()));
        let before = hero_health(&g, PlayerId::P2);
        g.attack(&timmy, "hero");
        g.expect_health("p2", before - 6);
    }

    #[test]
    fn r171_c52_twice_a_ready_unit_that_crosses_away_and_back_in_one_turn_is_sick_again() {
        let mut g = setup(json!({
            "p1": { "hand": [SILAS, SILAS], "mana": 10, "field": [{ "def": SEVEN_SEVEN, "lane": 5 }] },
            "p2": { "field": [{ "def": VANILLA, "lane": 3 }] },
        }));
        let traveller = unit_at(&g, "p1", 5);
        assert!(offered_attacks(&g, &traveller).contains(&"hero-p2".to_string()));

        g.play(SILAS, json!({ "zone": 1, "modes": ["right"] }));
        assert_eq!(g.card(&traveller).controller, PlayerId::P2);
        g.play(SILAS, json!({ "zone": 1, "modes": ["left"] }));

        assert_eq!(g.unit("p1", 5).map(|c| c.id), Some(traveller.id.clone()));
        expect_entered_now(&g, &traveller);
        g.expect_refused_with(|g| g.attack(&traveller, "hero"), SICK);
    }

    #[test]
    fn r171_c52_twice_a_charge_unit_that_attacked_and_made_the_round_trip_may_attack_once_more() {
        let mut g = setup(json!({
            "p1": { "hand": [SILAS, SILAS], "mana": 10, "field": [{ "def": TIMMY, "radiant": true, "lane": 5 }] },
            "p2": { "field": [{ "def": VANILLA, "lane": 3 }] },
        }));
        let timmy = unit_at(&g, "p1", 5);
        let before = hero_health(&g, PlayerId::P2);
        g.attack(&timmy, "hero");

        g.play(SILAS, json!({ "zone": 1, "modes": ["right"] }));
        g.play(SILAS, json!({ "zone": 1, "modes": ["left"] }));

        expect_entered_now(&g, &timmy);
        g.attack(&timmy, "hero");
        g.expect_health("p2", before - 12);
    }
}

/// R172 a stolen unit dies as its controller's
mod r172_a_stolen_unit_dies_as_its_controllers {
    use super::*;

    #[test]
    fn r172_c81_a_stolen_saintess_dies_for_the_thief_radiating_the_thiefs_units_and_goes_to_her_owners_graveyard() {
        let mut g = setup(json!({
            "p1": { "hand": [MIND_CONTROL, SORCERER], "mana": 10, "field": [{ "def": POINTMASTER, "lane": 5 }] },
            "p2": { "hand": [VANILLA], "field": [{ "def": SAINTESS, "lane": 2 }, { "def": POINTMASTER, "lane": 4 }] },
        }));
        let saintess = unit_at(&g, "p2", 2);
        let mine = unit_at(&g, "p1", 5);
        let theirs = unit_at(&g, "p2", 4);

        g.play(MIND_CONTROL, json!({ "targets": at(&saintess) }));
        g.play(SORCERER, json!({ "targets": at(&saintess) }));

        let died: Vec<&GameEvent> = g
            .events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == saintess.id))
            .collect();
        assert_eq!(died.len(), 1);
        assert!(
            matches!(died[0], GameEvent::Destroyed { owner: PlayerId::P2, .. }),
            "{:?} should match {{ owner: \"p2\" }}",
            died[0]
        );
        // Death: "Make your other Units Radiant" — "your" is the thief.
        assert!(g.card(&mine).radiant);
        assert!(!g.card(&theirs).radiant);
        // No Reborn since patch v0.1.1: off the field she is her owner's again (R12).
        assert!(g.unit("p1", 2).is_none());
        assert!(g.pile("p2", "graveyard").iter().any(|card| card.id == saintess.id));
    }

    #[test]
    fn r172_radiant_c3_a_stolen_right_house_defenders_death_summons_its_base_copy_on_the_thiefs_side() {
        let mut g = setup(json!({
            "p1": { "hand": [MIND_CONTROL, VANILLA], "field": [{ "def": POINTMASTER, "lane": 5 }], "library": LIBRARY },
            "p2": {
                "hand": [VANILLA],
                "field": [
                    { "def": RIGHT_HOUSE, "radiant": true, "lane": 2 },
                    { "def": VANILLA, "lane": 4 },
                    { "def": VANILLA, "lane": 5 },
                ],
                "library": LIBRARY,
            },
        }));
        let defender = unit_at(&g, "p2", 2);
        let first = unit_at(&g, "p2", 4);
        let second = unit_at(&g, "p2", 5);
        g.play(MIND_CONTROL, json!({ "targets": at(&defender) }));
        g.end_turn();

        // p2 breaks the shield, then kills it: it dies as p1's.
        g.attack(&first, &defender);
        g.attack(&second, &defender);

        let copy = g.unit("p1", 1);
        assert_eq!(copy.as_ref().map(|c| c.def_id.clone()), Some(RIGHT_HOUSE.to_string()));
        assert_eq!(copy.as_ref().map(|c| c.radiant), Some(false));
        assert_eq!(copy.as_ref().map(|c| c.controller), Some(PlayerId::P1));
        assert_eq!(g.unit("p1", 2).map(|c| c.id), Some(defender.id.clone()));
        assert_eq!(g.card(&defender).controller, PlayerId::P1);
        assert_eq!(g.card(&defender).owner, PlayerId::P2);
        for lane in 1..=3 {
            assert!(g.unit("p2", lane).is_none(), "p2 lane {lane} should be empty");
        }
    }
}

/// R61: a played permanent rotated onto the trap's side is not its own Fuse target
mod r61_a_played_permanent_rotated_onto_the_traps_side_is_not_its_own_fuse_target {
    use super::*;

    #[test]
    fn r61_r77_unlicensed_experimentation_fuses_a_played_silly_silas_that_crossed_to_its_side_onto_its_controllers_other_unit_never_onto_silas_himself_s8_c85()
     {
        // Seed chosen so that, with the played card wrongly among the candidates, the random pick lands
        // on Silas himself: the trap then fires, is consumed and fuses nothing although Pointmaster was
        // a legal target all along.
        let mut g = setup(json!({
            "seed": "ue-self-0",
            "p1": { "hand": [SILAS, VANILLA], "library": LIBRARY },
            "p2": {
                "hand": [VANILLA],
                "field": [{ "def": POINTMASTER, "lane": 2 }],
                "backrow": [{ "def": UNLICENSED, "lane": 3, "faceUp": false }],
                "library": LIBRARY,
            },
        }));
        let silas = g.card(SILAS).clone();
        let pointmaster = unit_at(&g, "p2", 2);

        // Silas enters p1's lane 5 and rotates right: he crosses to p2's lane 5 (§3.1), and
        // Pointmaster steps from p2's lane 2 to p2's lane 1.
        g.play(&silas, json!({ "zone": 5, "modes": ["right"] }));

        // p2's trap answers p1's played Unit (R17): "Fuse it onto a random permanent of yours of that
        // type". "It" is the played card, so the permanent it is fused onto is another one: the only
        // candidate is Pointmaster, and the fusion happens.
        assert!(g.events().iter().any(|event| event.event_type() == GameEventType::TrapFired));
        let fused: Vec<&GameEvent> = g
            .events()
            .iter()
            .filter(|event| event.event_type() == GameEventType::Fused)
            .collect();
        assert_eq!(fused.len(), 1);
        assert!(
            matches!(fused[0], GameEvent::Fused { result_instance_id, .. } if *result_instance_id == pointmaster.id),
            "{:?} should match {{ resultInstanceId: {:?} }}",
            fused[0],
            pointmaster.id
        );
        assert!(g.unit("p2", 5).is_none());
    }
}

/// R14: radiant Silly Silas bounces only the cards that would move to the opponent
mod r14_radiant_silly_silas_bounces_only_the_cards_that_would_move_to_the_opponent {
    use super::*;

    #[test]
    fn r14_r171_radiant_silly_silas_still_takes_the_opponents_card_that_crosses_to_its_controllers_side_s8_c52_radiant_s8_conventions()
     {
        let mut g = setup(json!({
            "p1": { "hand": [{ "def": SILAS, "radiant": true }, VANILLA], "library": LIBRARY },
            "p2": { "hand": [VANILLA], "field": [{ "def": VANILLA, "lane": 1 }], "library": LIBRARY },
        }));
        let inbound = unit_at(&g, "p2", 1);

        // Right from p1's seat: p2's lane 1 steps to p1's lane 1. That card moves to p1, not "to the
        // opponent", so the radiant clause does not touch it and the base clause keeps it: it crosses
        // and changes control (§8 Conventions: every base clause the radiant cell does not restate is
        // kept), entering p1's side this turn (R171).
        g.play(SILAS, json!({ "zone": 3, "modes": ["right"] }));

        assert_eq!(g.unit("p1", 1).map(|c| c.id), Some(inbound.id.clone()));
        assert_eq!(g.card(&inbound).controller, PlayerId::P1);
        assert_eq!(g.card(&inbound).owner, PlayerId::P2);
        assert_eq!(g.card(&inbound).summoned_turn, Some(g.state().turn));
        let expected = json!({
            "type": "controlChanged",
            "instanceId": inbound.id,
            "controller": "p1",
            "row": "units",
            "lane": 1,
        });
        assert!(
            g.last_events()
                .iter()
                .any(|event| serde_json::to_value(event).unwrap() == expected),
            "the last step's events should hold {expected}"
        );
    }
}
