//! #96 My Pawn (SPEC §8.5, §4.2 step 4, §6.3 "Cancel an attack", §10.7's AI bullet, R44, R84,
//! R276, R283). Trap, cost 1, Mythic.
//!   Base:    "When the opponent declares an attack that would be lethal to your hero: cancel it,
//!             and an AI plays the rest of their turn with random legal actions"
//!   Radiant: "When the opponent declares an attack that would be lethal to your hero: cancel it,
//!             destroy the attacker, and an AI plays the rest of their turn with random legal
//!             actions" (§8's cell "Also destroy the attacker"; R276 gave the card its Radiant face).
//! Trigger in §4.2 step 4 trap window on attackDeclared against hero (R100, §10.3; R61 arms until lethal).
//! Subsystems calculate lethal damage (subsystems/lethal.rs, R44; forced attacks ignored, R121, R53).
//! Effects in order (CLAUDE.md rule 5): cancel_attack, then (if radiant) destroy attacker (§6.3, R46, R283, R174, R102), then ai_plays_out_turn (R84, R152).
//! Glow indicates lethal attacker on board (R662).

use jackioh_engine::effects::{ai_plays_out_turn, cancel_attack};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-096";

/// R61, R44: armed on opponent declaration lethal to controller's hero.
fn would_be_lethal(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::AttackDeclared {
        attacker_id,
        target_id,
        forced,
        ..
    } = event
    else {
        return false;
    };
    // §4.2's forced attacks are declared by the compelling effect, not by the opponent (R53).
    if *forced {
        return false;
    }

    let Some(attacker) = find_instance(ctx.state, attacker_id) else {
        return false;
    };
    // "the opponent declares": a trap never answers its own controller's attack.
    if attacker.controller != opponent_of(ctx.controller) {
        return false;
    }

    let Some(target) = attack_target_of(ctx.state, target_id) else {
        return false;
    };
    // R44: counts Trample excess towards trap controller's hero.
    if subsystems::defending_hero(&target) != ctx.controller {
        return false;
    }

    subsystems::is_lethal(ctx.state, attacker, &target)
}

/// R283: radiant text adds `destroys_attacker`; shared `when` predicate.
fn my_pawn(destroys_attacker: bool, when: &TriggerWhen) -> TriggerDef {
    // §8.5's clauses in its order. `player: "enemy"` is the attacker's side relative to the trap's
    // controller, which the predicate above has already established.
    let mut trigger = TriggerDef::new("my-pawn", &[GameEventType::AttackDeclared], move |_ctx, _event| {
        let mut cancel = json!({});
        if destroys_attacker {
            cancel["destroyAttacker"] = json!(true);
        }
        let mut playout = json!({ "player": "enemy" });
        if destroys_attacker {
            playout["settleFirst"] = json!(true);
        }
        vec![cancel_attack(json_as(cancel)), ai_plays_out_turn(json_as(playout))]
    });
    trigger.when = Some(when.clone());
    trigger
}

/// R662: armed while an enemy unit on the field could swing for lethal at its controller's hero.
fn condition_met(ctx: ConditionContext<'_>) -> bool {
    ctx.zone == ConditionZone::Field && !lethal_attackers_of(ctx.state, ctx.controller).is_empty()
}

pub fn script() -> CardScripts {
    let when: TriggerWhen = Arc::new(would_be_lethal);
    CardScripts {
        base: Script {
            triggers: vec![my_pawn(false, &when)],
            condition_met: Some(condition_hook(condition_met)),
            ..Script::default()
        },
        radiant: Script {
            triggers: vec![my_pawn(true, &when)],
            condition_met: Some(condition_hook(condition_met)),
            ..Script::default()
        },
    }
}

// #96 My Pawn — SPEC §8.5 row 96, BUILD M4-T4 row 96 (R44, R46, R84, R276, R283; §4.2 step 4, §6.3, §10.7).
// Tests check `when` predicate directly for lethal boundary and R662 yellow glow (`conditionMet`).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::events_json;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MY_PAWN: &str = "core-096";
    /// #13 Jlockeed Shredder-10, 6/10 with no keywords: a plain 6-damage swing.
    const ATTACKER: &str = "core-013";
    /// #73 Anti-oneshot Armor: `staticFlags.antiOneshot`, so §4.4 step 3 clamps to 5 (radiant 3).
    const ANTI_ONESHOT: &str = "core-073";
    /// A 1/1 body to be attacked, so Trample excess has something to spill past (§4.4 step 9).
    const SMALL: &str = "core-t-felinor";

    use crate::js;

    /// The only trigger the card registers; `when` is where every arming condition lives (R61).
    fn trigger(face: &str) -> TriggerDef {
        let scripts = script();
        let chosen = if face == "base" { scripts.base } else { scripts.radiant };
        chosen
            .triggers
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("#96's {face} face registers no trigger"))
    }

    fn attack_declared(attacker_id: &str, target_id: &str, forced: bool) -> GameEvent {
        GameEvent::AttackDeclared {
            attacker_id: attacker_id.to_string(),
            target_id: target_id.to_string(),
            forced,
                instead_of: None,
        }
    }

    /// Builds `when` context as in `traps::fire_trap`.
    fn arms(s: &Scenario, event: &GameEvent) -> bool {
        let trap = s.card(MY_PAWN).clone();
        let mut state = s.state().clone();
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            Some(&trap),
            HookOptions {
                controller: Some(trap.controller),
                ..Default::default()
            },
        );
        let when = trigger("base").when.expect("#96's trigger has no `when` predicate");
        when(&mut ctx, event)
    }

    #[derive(Default)]
    struct Swing {
        health: i32,
        armor: Option<i32>,
        backrow: Vec<Value>,
        field: Option<Vec<Value>>,
    }

    /// p1 holds the trap; p2 is active and swings at p1's hero for 6.
    fn swing(p1: Swing) -> Scenario {
        let mut side = json!({ "health": p1.health });
        if let Some(armor) = p1.armor {
            side["armor"] = json!(armor);
        }
        let mut backrow = vec![json!(MY_PAWN)];
        backrow.extend(p1.backrow);
        side["backrow"] = json!(backrow);
        if let Some(field) = p1.field {
            side["field"] = json!(field);
        }
        scenario(json!({
            "seed": "my-pawn",
            "active": "p2",
            "p1": side,
            "p2": { "field": [ATTACKER] },
        }))
    }

    fn id_of(s: &Scenario, card: &str) -> String {
        s.card(card).id.clone()
    }

    /// Granting keyword on a fixture writes state directly (CLAUDE.md rule 5).
    fn grant_trample(s: &mut Scenario, instance_id: &str) {
        find_instance_mut(s.state_mut(), instance_id)
            .expect("the attacker is on the field")
            .granted_keywords = vec![json_as(json!({ "kind": "Trample" }))];
    }

    mod n96_my_pawn_the_trigger_it_registers {
        use super::*;

        #[test]
        fn r61_the_arming_condition_is_a_when_predicate_not_an_empty_run() {
            crate::register_all();
            // `traps.rs`: "`run` returning `[]` is a trap that fired for nothing — it can never mean
            // 'this event was not mine'", so a non-lethal swing must be refused before `run`.
            assert!(trigger("base").when.is_some());
        }

        #[test]
        fn r276_the_radiant_face_registers_its_own_trigger_on_the_same_event_and_the_same_condition() {
            crate::register_all();
            let scripts = script();
            let base = &scripts.base.triggers[0];
            let radiant = &scripts.radiant.triggers[0];
            assert!(!Arc::ptr_eq(&radiant.run, &base.run));
            assert_eq!(scripts.radiant.triggers.len(), 1);
            assert_eq!(radiant.on, base.on);
            let (Some(radiant_when), Some(base_when)) = (&radiant.when, &base.when) else {
                panic!("both faces carry the `when` predicate");
            };
            assert!(Arc::ptr_eq(radiant_when, base_when));
        }
    }

    mod n96_my_pawn_lethal_detection_r44 {
        use super::*;

        #[test]
        fn r44_fires_when_the_projected_damage_is_at_least_the_hero_s_health() {
            crate::register_all();
            let s = swing(Swing { health: 6, ..Swing::default() });
            assert!(arms(&s, &attack_declared(&id_of(&s, ATTACKER), "hero-p1", false)));
        }

        #[test]
        fn r44_does_not_fire_when_the_hero_would_survive_by_one() {
            crate::register_all();
            let s = swing(Swing { health: 7, ..Swing::default() });
            assert!(!arms(&s, &attack_declared(&id_of(&s, ATTACKER), "hero-p1", false)));
        }

        #[test]
        fn r44_counts_armor_1_armor_makes_the_same_6_damage_swing_survivable_at_6_health() {
            crate::register_all();
            let armored = swing(Swing { health: 6, armor: Some(1), ..Swing::default() });
            assert!(!arms(&armored, &attack_declared(&id_of(&armored, ATTACKER), "hero-p1", false)));

            // 6 − 1 Armor = 5, which is exactly lethal at 5 (§4.4 step 2, §4.5 step 2).
            let exact = swing(Swing { health: 5, armor: Some(1), ..Swing::default() });
            assert!(arms(&exact, &attack_declared(&id_of(&exact, ATTACKER), "hero-p1", false)));
        }

        #[test]
        fn r44_counts_the_anti_oneshot_cap_a_6_damage_swing_projects_4() {
            crate::register_all();
            let capped = swing(Swing { health: 5, backrow: vec![json!(ANTI_ONESHOT)], ..Swing::default() });
            assert!(!arms(&capped, &attack_declared(&id_of(&capped, ATTACKER), "hero-p1", false)));

            let exact = swing(Swing { health: 4, backrow: vec![json!(ANTI_ONESHOT)], ..Swing::default() });
            assert!(arms(&exact, &attack_declared(&id_of(&exact, ATTACKER), "hero-p1", false)));
        }

        #[test]
        fn r44_counts_the_radiant_anti_oneshot_cap_of_3() {
            crate::register_all();
            let s = scenario(json!({
                "seed": "my-pawn-cap-3",
                "active": "p2",
                "p1": { "health": 4, "backrow": [MY_PAWN, { "def": ANTI_ONESHOT, "radiant": true }] },
                "p2": { "field": [ATTACKER] },
            }));
            assert!(!arms(&s, &attack_declared(&id_of(&s, ATTACKER), "hero-p1", false)));

            let exact = scenario(json!({
                "seed": "my-pawn-cap-3",
                "active": "p2",
                "p1": { "health": 3, "backrow": [MY_PAWN, { "def": ANTI_ONESHOT, "radiant": true }] },
                "p2": { "field": [ATTACKER] },
            }));
            // ANTI_ONESHOT_CAP.radiant is 3, so 3 is exactly lethal and 4 is not.
            assert!(arms(&exact, &attack_declared(&id_of(&exact, ATTACKER), "hero-p1", false)));
        }

        #[test]
        fn r44_counts_trample_excess_from_an_attack_on_a_unit_and_nothing_without_trample() {
            crate::register_all();
            let mut s = swing(Swing { health: 5, field: Some(vec![json!(SMALL)]), ..Swing::default() });
            let attacker = id_of(&s, ATTACKER);
            let victim = id_of(&s, SMALL);

            // §4.4 step 9 sends only the excess on: 6 − the 1/1's 1 health = 5, exactly lethal at 5.
            grant_trample(&mut s, &attacker);
            assert!(arms(&s, &attack_declared(&attacker, &victim, false)));

            let plain = swing(Swing { health: 5, field: Some(vec![json!(SMALL)]), ..Swing::default() });
            // No Trample: the hit stops on the unit, so no hero damage is projected at all.
            assert!(!arms(&plain, &attack_declared(&id_of(&plain, ATTACKER), &id_of(&plain, SMALL), false)));
        }
    }

    mod n96_my_pawn_whose_attack_it_answers {
        use super::*;

        #[test]
        fn s8_5_the_opponent_declares_it_never_answers_its_own_controller_s_attack() {
            crate::register_all();
            let s = scenario(json!({
                "seed": "my-pawn-own",
                "active": "p1",
                "p1": { "health": 6, "backrow": [MY_PAWN], "field": [ATTACKER] },
                "p2": { "health": 6 },
            }));
            // p1's own 6-damage swing would be lethal to p2, and p1's trap is not interested.
            assert!(!arms(&s, &attack_declared(&id_of(&s, ATTACKER), "hero-p2", false)));
        }

        #[test]
        fn r121_a_forced_attack_is_declared_by_the_effect_not_the_player_so_a_trigger_keyed_to_an_opponent_s_declaration_does_not_arm() {
            crate::register_all();
            let s = swing(Swing { health: 6, ..Swing::default() });
            // §4.2, R53: forced attack skips steps 1-3, spends no exertion.
            assert!(!arms(&s, &attack_declared(&id_of(&s, ATTACKER), "hero-p1", true)));
        }

        #[test]
        fn r44_lethal_to_your_hero_the_hero_the_projection_reaches_is_the_one_that_matters() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "my-pawn-other-hero",
                "active": "p2",
                // p1 holds the trap at 1 health, so anything reaching it would be lethal.
                "p1": { "health": 1, "backrow": [MY_PAWN] },
                // §4.4 step 9: Trample excess goes to target's controller's hero.
                "p2": { "health": 30, "field": [ATTACKER, SMALL] },
            }));
            let attacker = s.unit(P2, 1);
            let victim = s.unit(P2, 2);
            assert!(attacker.is_some());
            assert!(victim.is_some());
            if let Some(found) = &attacker {
                grant_trample(&mut s, &found.id);
            }

            let attacker_id = attacker.map(|card| card.id).unwrap_or_default();
            let victim_id = victim.map(|card| card.id).unwrap_or_default();
            assert!(!arms(&s, &attack_declared(&attacker_id, &victim_id, false)));
        }
    }

    // R44, R84, R152: base trap cancel, lockout, and AI play.

    // Radiant: "cancel it, destroy the attacker, and an AI plays the rest of their turn" (R283)

    /// #68, a 5/5 with nothing that fires in combat: the plain lethal swing at a 5-health hero.
    const SORCERER: &str = "core-068";
    /// #56 Jilliax: its radiant face is a 6/4 with Rush, Taunt, Lifesteal, Divine Shield and Reborn.
    const JILLIAX: &str = "core-056";
    /// #66 The Rock, 10/10 Indestructible: the one Indestructible unit in Core.
    const ROCK: &str = "core-066";
    /// Cards for the AI's turn, as the other My Pawn tests give it (my_pawn.rs).
    const STOCKPILE: &str = "core-005";
    const TIMMY: &str = "core-011";
    const GIGA: &str = "core-029";
    /// #3 Right-house defender, 1/1 Taunt, Divine Shield, Reborn: a lethal swing at a 1-health hero.
    const RIGHT_HOUSE: &str = "core-003";
    /// #89 Corpse Eater: in hand, it gains the attack and max health of each unit that dies (R38).
    const CORPSE_EATER: &str = "core-089";
    /// #20 Pointmaster: its radiant face is a 14/4 with First Strike and Divine Shield, and no Reborn.
    const POINTMASTER: &str = "core-020";

    /// p1 swings `attacker` from lane 1 at p2's hero, which is at `health` behind a face-down My Pawn of
    /// the given face. p1 holds cards for the AI turn the trap hands over.
    fn pawn_game(attacker: Value, health: i32, face: &str) -> (Scenario, CardInstance) {
        let mut s = scenario(json!({
            "seed": "my-pawn-r283",
            "p1": { "field": [attacker], "hand": [STOCKPILE, TIMMY], "library": [GIGA, GIGA, GIGA] },
            "p2": {
                "health": health,
                "hand": [STOCKPILE],
                "backrow": [{ "def": MY_PAWN, "lane": 1, "faceUp": false, "radiant": face == "radiant" }],
                "library": [GIGA, GIGA],
            },
        }));
        let unit = s.unit(P1, 1).unwrap_or_else(|| panic!("p1 should have an attacker in lane 1"));
        s.attack(&unit, "hero");
        (s, unit)
    }

    /// The event types from the trap's cancel on, `count` of them: what the trap's list did first.
    fn from_cancel(s: &Scenario, count: usize) -> Vec<String> {
        let all = events_json(s);
        let at = all
            .iter()
            .position(|event| event["type"] == "attackCancelled")
            .unwrap_or_else(|| panic!("no attack was canceled"));
        all[at..]
            .iter()
            .take(count)
            .map(|event| event["type"].as_str().unwrap_or_default().to_string())
            .collect()
    }

    fn events_on(s: &Scenario, id: &str, kind: &str) -> usize {
        events_json(s)
            .iter()
            .filter(|event| event["type"] == kind && event.get("instanceId").is_some_and(|found| found == id))
            .count()
    }

    fn any_event(s: &Scenario, kind: &str) -> bool {
        events_json(s).iter().any(|event| event["type"] == kind)
    }

    fn turn_went_on(s: &Scenario, from: PlayerId) {
        // The AI played the rest of the turn out and ended it (R44, R152).
        assert!(any_event(s, "turnEnded"));
        assert_ne!(s.state().active, from);
        assert!(!s.state().players[from].ai_turn);
    }

    mod n96_my_pawn_radiant_r283 {
        use super::*;

        #[test]
        fn r283_cancels_the_lethal_attack_then_destroys_the_attacker_and_the_check_collects_it_before_the_ai_takes_the_turn() {
            crate::register_all();
            let (mut s, attacker) = pawn_game(json!(SORCERER), 5, "radiant");

            // Canceled: no combat, so the hero took nothing (R44).
            s.expect_health(P2, 5);
            assert_eq!(events_json(&s).iter().filter(|event| event["type"] == "attackCancelled").count(), 1);
            assert!(!any_event(&s, "damage"));

            // The attacker is destroyed, after the cancel, and collected right then — before any event
            // of the AI's turn.
            assert_eq!(from_cancel(&s, 3), vec!["attackCancelled", "destroyed", "enteredGraveyard"]);
            assert_eq!(events_on(&s, &attacker.id, "destroyed"), 1);
            s.expect_in_zone(&attacker, "graveyard");

            turn_went_on(&s, P1);
        }

        #[test]
        fn r283_the_destroy_is_ordinary_a_reborn_attacker_comes_back_before_the_ai_takes_the_turn() {
            crate::register_all();
            // A radiant Jilliax's Divine Shield does not stop a destroy (§6.3), and its Reborn answers it.
            let (mut s, attacker) = pawn_game(json!({ "def": JILLIAX, "radiant": true }), 6, "radiant");

            s.expect_health(P2, 6);
            // §4.5 step 4: Reborn returns it to the zone it reserved (R64), at 1 health, straight after
            // the collection and ahead of the AI turn.
            assert_eq!(from_cancel(&s, 3), vec!["attackCancelled", "destroyed", "summoned"]);
            assert_eq!(s.card(&attacker).reborn_spent, Some(true));
            s.expect_in_zone(&attacker, "field");
            turn_went_on(&s, P1);
        }

        #[test]
        fn r283_r46_an_indestructible_attacker_is_knocked_down_instead_and_the_attack_is_canceled_either_way() {
            crate::register_all();
            // #66 The Rock: a 10/10 Indestructible, in Attack Position, with no Taunt (R347 takes any).
            let (mut s, attacker) = pawn_game(json!(ROCK), 10, "radiant");

            // Canceled: the 10 never landed.
            s.expect_health(P2, 10);
            assert!(!events_json(&s)
                .iter()
                .any(|event| event["type"] == "damage" && event["sourceId"] == attacker.id.as_str()));

            // R46, R91: destroy mark on Indestructible does not kill; reports nothing.
            assert_eq!(from_cancel(&s, 1), vec!["attackCancelled"]);
            assert_eq!(events_on(&s, &attacker.id, "positionSwitched"), 0);
            assert_eq!(events_on(&s, &attacker.id, "keywordGranted"), 0);
            assert_eq!(events_on(&s, &attacker.id, "destroyed"), 0);
            s.expect_in_zone(&attacker, "field");
            turn_went_on(&s, P1);
        }

        #[test]
        fn r283_destroy_is_not_damage_an_attacker_with_divine_shield_is_destroyed_all_the_same_its_shield_unspent() {
            crate::register_all();
            // §6.1, §6.3: Divine Shield negates damage, not destroy. Radiant #20 has no Reborn.
            let printed: Vec<Value> = js(&crate::card_def(POINTMASTER))["radiant"]["keywords"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|keyword| keyword["kind"].clone())
                .collect();
            assert!(printed.contains(&json!("Divine Shield")));
            assert!(!printed.contains(&json!("Reborn")));
            let (mut s, attacker) = pawn_game(json!({ "def": POINTMASTER, "radiant": true }), 14, "radiant");

            // Canceled: the 14 never landed.
            s.expect_health(P2, 14);
            assert!(!any_event(&s, "damage"));

            // Destroyed after the cancel and collected before the AI's turn, the shield never spent.
            assert_eq!(from_cancel(&s, 3), vec!["attackCancelled", "destroyed", "enteredGraveyard"]);
            assert_eq!(events_on(&s, &attacker.id, "destroyed"), 1);
            assert_eq!(events_on(&s, &attacker.id, "divineShieldLost"), 0);
            s.expect_in_zone(&attacker, "graveyard");
            turn_went_on(&s, P1);
        }

        #[test]
        fn r283_the_attacker_s_death_is_answered_before_the_ai_takes_the_turn_a_corpse_eater_in_the_ai_s_hand_eats_it_first() {
            crate::register_all();
            // R212, #89: death dispatched and hand trigger run before AI acts.
            let mut s = scenario(json!({
                "seed": "my-pawn-r283-eater",
                "p1": {
                    "field": [SORCERER],
                    "hand": [CORPSE_EATER, STOCKPILE],
                    "library": [GIGA, GIGA, GIGA],
                    "mana": 4,
                },
                "p2": {
                    "health": 5,
                    "hand": [STOCKPILE],
                    "backrow": [{ "def": MY_PAWN, "lane": 1, "faceUp": false, "radiant": true }],
                    "library": [GIGA, GIGA],
                },
            }));
            let eater = s.card(CORPSE_EATER).clone();
            let attacker = s.unit(P1, 1).unwrap_or_else(|| panic!("p1 should have an attacker in lane 1"));
            s.attack(&attacker, "hero");

            // The Sorcerer's 5 attack and 5 max health, gained while the Eater was still in hand (R38, R89).
            assert_eq!(js(&s.card(&eater).buffs), json!({ "attack": 5, "health": 5 }));
            let all = events_json(&s);
            let index_of = |kind: &str| -> isize {
                all.iter()
                    .position(|event| event["type"] == kind)
                    .map_or(-1, |at| at as isize)
            };
            let fed = all
                .iter()
                .position(|event| event["type"] == "buffed" && event["instanceId"] == eater.id.as_str())
                .map_or(-1, |at| at as isize);
            let canceled_at = index_of("attackCancelled");
            let ai_acts = all
                .iter()
                .enumerate()
                .position(|(at, event)| {
                    (at as isize) > canceled_at && (event["type"] == "cardPlayed" || event["type"] == "turnEnded")
                })
                .map_or(-1, |at| at as isize);
            assert!(fed > index_of("destroyed"));
            assert!(fed < ai_acts);
            turn_went_on(&s, P1);
        }

        #[test]
        fn r283_r174_a_radiant_my_pawn_fused_onto_a_radiant_my_pawn_destroys_the_attacker_once_not_its_reborn_body_again() {
            crate::register_all();
            // #85, R61, R77, R102: fused trap runs both texts in sequence.
            let mut s = scenario(json!({
                "seed": "my-pawn-r283-fused",
                "active": "p2",
                "turn": 10,
                "p1": {
                    "health": 1,
                    "field": ["core-008"],
                    // A library, so p1's own draw after the AI turn takes no fatigue and the game goes on.
                    "library": [GIGA, GIGA],
                    "backrow": [
                        { "def": MY_PAWN, "faceUp": false, "radiant": true },
                        { "def": "core-085", "faceUp": false },
                    ],
                },
                "p2": { "hand": [MY_PAWN], "field": [RIGHT_HOUSE], "mana": 4 },
            }));
            s.play(MY_PAWN, json!({}));
            let fused = s.backrow(P1, 1).map(|card| card.def_id).unwrap_or_default();
            assert!(fused.ends_with("core-096+core-096"));

            let attacker = s
                .unit(P2, 1)
                .unwrap_or_else(|| panic!("p2 should have its Right-house defender in lane 1"));
            s.attack(&attacker, "hero");

            // The first half destroyed it and it came back through Reborn; the second half finds the
            // attacker gone from the stay the declaration named, and leaves the new body alone (R174).
            assert_eq!(events_on(&s, &attacker.id, "destroyed"), 1);
            s.expect_in_zone(&attacker, "field");
            assert_eq!(s.card(&attacker).reborn_spent, Some(true));
        }

        #[test]
        fn the_base_face_does_not_destroy_the_attacker_it_only_cancels_and_hands_the_turn_over() {
            crate::register_all();
            let (mut s, attacker) = pawn_game(json!(SORCERER), 5, "base");

            s.expect_health(P2, 5);
            assert_eq!(events_on(&s, &attacker.id, "destroyed"), 0);
            s.expect_in_zone(&attacker, "field");
            turn_went_on(&s, P1);
        }

        #[test]
        fn r61_a_non_lethal_swing_leaves_the_radiant_trap_armed_and_the_attacker_standing() {
            crate::register_all();
            let (mut s, attacker) = pawn_game(json!(SORCERER), 30, "radiant");

            s.expect_health(P2, 25);
            assert!(!any_event(&s, "trapFired"));
            assert_eq!(s.backrow(P2, 1).and_then(|card| card.face_up), Some(false));
            s.expect_in_zone(&attacker, "field");
        }
    }

    mod n96_my_pawn_glows_while_an_enemy_unit_could_swing_for_lethal_r662_r44 {
        use super::*;

        const POINTMASTER: &str = "core-020"; // 7/1 First Strike

        fn face_of(radiant: bool) -> &'static str {
            if radiant { "radiant" } else { "base" }
        }

        /// R662: at 7 health it glows for its controller only, and the swing is canceled.
        fn glows_at_7(radiant: bool) {
            let face = face_of(radiant);
            let mut s = scenario(json!({
                "seed": format!("r662-096-{face}-on"),
                "active": "p2",
                "p1": { "backrow": [{ "def": "core-096", "radiant": radiant }], "health": 7 },
                "p2": { "field": [POINTMASTER], "hand": ["core-010"] },
            }));
            assert!(backrow_glows(&s, 1, P1));
            assert!(!opponent_sees_glow(&s, 1, P1));

            // The rest of the turn is played at random afterwards, so the cancel is what is read here.
            s.attack(POINTMASTER, "hero");
            assert!(any_event(&s, "trapFired"));
        }

        /// R662: at 8 health it does not glow, and the swing lands.
        fn dark_at_8(radiant: bool) {
            let face = face_of(radiant);
            let mut s = scenario(json!({
                "seed": format!("r662-096-{face}-off"),
                "active": "p2",
                "p1": { "backrow": [{ "def": "core-096", "radiant": radiant }], "health": 8 },
                "p2": { "field": [POINTMASTER], "hand": ["core-010"] },
            }));
            assert!(!backrow_glows(&s, 1, P1));

            s.attack(POINTMASTER, "hero");
            assert!(!any_event(&s, "trapFired"));
            s.expect_health(P1, 1);
        }

        /// R662: on its controller's own turn it glows for the blow on the board.
        fn glows_on_own_turn(radiant: bool) {
            let face = face_of(radiant);
            let s = scenario(json!({
                "seed": format!("r662-096-{face}-mine"),
                "p1": { "backrow": [{ "def": "core-096", "radiant": radiant }], "health": 7, "hand": ["core-010"] },
                "p2": { "field": [POINTMASTER] },
            }));
            assert!(backrow_glows(&s, 1, P1));
        }

        #[test]
        fn r662_base_at_7_health_it_glows_for_its_controller_only_and_the_swing_is_canceled() {
            crate::register_all();
            glows_at_7(false);
        }

        #[test]
        fn r662_base_at_8_health_it_does_not_glow_and_the_swing_lands() {
            crate::register_all();
            dark_at_8(false);
        }

        #[test]
        fn r662_base_on_its_controller_s_own_turn_it_glows_for_the_blow_on_the_board() {
            crate::register_all();
            glows_on_own_turn(false);
        }

        #[test]
        fn r662_radiant_at_7_health_it_glows_for_its_controller_only_and_the_swing_is_canceled() {
            crate::register_all();
            glows_at_7(true);
        }

        #[test]
        fn r662_radiant_at_8_health_it_does_not_glow_and_the_swing_lands() {
            crate::register_all();
            dark_at_8(true);
        }

        #[test]
        fn r662_radiant_on_its_controller_s_own_turn_it_glows_for_the_blow_on_the_board() {
            crate::register_all();
            glows_on_own_turn(true);
        }
    }
}
