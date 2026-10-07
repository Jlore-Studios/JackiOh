//! #96 My Pawn (SPEC §8.5, §4.2 step 4, §6.3 "Cancel an attack", §10.7's AI bullet, R44, R84,
//! R276, R283). Trap, cost 1, Mythic.
//!   Base:    "When the opponent declares an attack that would be lethal to your hero: cancel it,
//!             and an AI plays the rest of their turn with random legal actions"
//!   Radiant: "When the opponent declares an attack that would be lethal to your hero: cancel it,
//!             destroy the attacker, and an AI plays the rest of their turn with random legal
//!             actions" (§8's cell "Also destroy the attacker"; R276 gave the card its Radiant face).
//!
//! WHAT FIRES IT. §4.2 step 4: "Declaring the attack has now spent the attacker's exertion, before
//! any damage. Trap window: My Pawn checks whether the hit would be lethal and, if so, cancels the
//! attack; the exertion is not given back, so the attack is gone either way (R44)". The event that
//! opens that window is `attackDeclared`, which `combat::declare_attack` emits after spending the
//! exertion and hands to `traps::run_trap_window` before it resolves any combat, so this trigger
//! watches exactly that one event. R100 keeps the window's delivery to the window: the declaration
//! is not offered to §10.3's immediate check as well, so this trap answers one swing once.
//!
//! ARMING (R61, `traps.rs`). "`run` returning `[]` is a trap that fired for nothing — it can never
//! mean 'this event was not mine'", so every condition that must leave My Pawn face-down and armed
//! lives in the `when` predicate: the opponent's declaration, aimed at a hero that is mine, for
//! enough damage to end the game. A non-lethal swing therefore leaves the trap set, which is the
//! whole point of the card. Consuming it is not this file's business either — `fireTrap` emits
//! `trapFired`, runs the trigger, runs the state check and consumes the trap.
//!
//! LETHAL IS THE SUBSYSTEM'S (R44). "Lethal = projected damage to the hero after Armor and the cap,
//! Trample excess from an attack on a unit included, ≥ health". That calculation is
//! `subsystems/lethal.rs` (`is_lethal`, `projected_damage`, `projected_hero_damage`, `defending_hero`),
//! and this card only hands it an attacker and an `AttackTarget`. Nothing here re-reads §4.4.
//!
//! FORCED ATTACKS DO NOT FIRE IT. §4.2's last paragraph: a forced attack (Moths to the Flame, Bear
//! Honeypot) "skips steps 1 to 3" and spends no exertion, and R53 gives each one its own combat and
//! state check. The card's condition is "when the OPPONENT DECLARES an attack", and a forced attack
//! is declared by the effect that compels it — usually the defender's own card, on the defender's
//! own turn, where "an AI plays the rest of THEIR turn" names nobody. `forced` is on the event for
//! exactly this kind of distinction, so the predicate reads it (R121). `combat::force_attack` opens no
//! window at all for the same reason, so there is nothing for this trap to cancel there either.
//!
//! BUILDING THE `AttackTarget`. The event carries ids only (`attackerId`, `targetId`), and the
//! `hero-<player>` spelling is `combat.rs`'s. So the reverse of it is `combat::attack_target_of`, which
//! that module exports for exactly this: §4.2 step 5 reads a paused declaration back through it
//! too, so the card and the engine agree on what an id means by sharing one reader rather than by
//! each parsing the string.
//!
//! THE BODY (§8.5 #96 "cancel it, and an AI plays the rest of their turn with random legal actions")
//! is the two verbs of `crates/engine/src/effects/combat.rs`, in the row's order. Neither is
//! implemented here: a card file cannot cancel combat or drive the reducer itself (CLAUDE.md rules 4
//! and 5). `cancel_attack` marks the open `state.declaredAttack` cancelled, so §4.2 step 5 resolves
//! no combat and `attackCancelled` is emitted in its place; `ai_plays_out_turn` sets the `aiTurn`
//! lockout on the attacking player and hands the rest of their turn to §10.7's policy through
//! `subsystems::ai_policy::play_out_turn` (R84: never `concede`, `offerDraw` or `answerDraw`; R152: the
//! lockout ends at the cleanup of the turn it took).
//!
//! ORDER MATTERS, and it is the row's own. `cancel_attack` first, because `ai_plays_out_turn` drives
//! `reduce`, which clones the state, and the AI's own actions can open and close declarations of
//! their own — the attack this trap answers has to be cancelled while it is still the open one.
//! `ai_plays_out_turn` last for a second reason `effects/combat.rs` spells out: the playout replaces
//! every instance in the state, so no effect after it may hold a `CardInstance` read before it.
//!
//! THE RADIANT FACE (R283) adds "destroy the attacker", after the cancel and before the AI turn. It is
//! an ordinary §6.3 destroy — a mark the state check collects — so an Indestructible attacker is
//! knocked down instead (R46) and a Reborn one comes back, and the attack is cancelled either way
//! (R44). It rides on the cancel (`cancel_attack({ destroyAttacker })`), which names the attacker of
//! the declaration it cancels, on the stay it declared from (R174), and happens only where the cancel
//! does: a Radiant My Pawn fused onto a Radiant My Pawn runs its second half once the first has played
//! the turn out and the window has closed (R102), and so destroys the Reborn body of nothing.
//! R283 has the state check collect it before the AI takes the turn: the destroy is in its place in
//! the list, and `ai_plays_out_turn`'s `settleFirst` runs the check before the playout's first action,
//! so the AI acts from a board the attacker has already left. The base face has nothing to settle
//! and keeps the playout exactly as it was.
//!
//! THE GLOW (R662). The trap lights up on its controller's field while an enemy unit acting on the
//! field would deal lethal damage to their hero if it attacked it now (`lethal_attackers_of`, the same
//! `subsystems::is_lethal` the trigger asks). It says the blow is on the board, on either turn, not
//! that it can be declared this moment; stats and health are public. The same on both faces.

use jackioh_engine::effects::{ai_plays_out_turn, cancel_attack};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-096";

/// "When the opponent declares an attack that would be lethal to your hero". Every clause that must
/// leave the trap armed is here (R61): a declaration (not a forced attack), by the opponent, whose
/// projection lands on this trap's controller's hero, for at least that hero's health (R44). The
/// condition is the same on both faces.
fn would_be_lethal(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::AttackDeclared {
        attacker_id,
        target_id,
        forced,
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
    // R44 counts Trample excess from an attack on a unit, so the hero at risk is the projection's,
    // not the declared target: "lethal to YOUR hero" is that hero being this trap's controller.
    if subsystems::defending_hero(&target) != ctx.controller {
        return false;
    }

    subsystems::is_lethal(ctx.state, attacker, &target)
}

/// `destroys_attacker` is the whole of the radiant text (R283). `when` is the one predicate both
/// faces share (TS handed both triggers the same `wouldBeLethal` object).
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

// #96 My Pawn — SPEC §8.5, §4.2 step 4, §6.3 "Cancel an attack", §10.7's AI bullet, R44, R46, R84,
// R276, R283.
// BUILD M4-T4 row 96: "Lethal detection accounts for armor and the cap (R44); attack cancelled;
// AI finishes the turn deterministically from the seed; opponent's actions rejected until end of
// turn; radiant destroys the attacker with the cancel, before the AI turn, an Indestructible one
// knocked down, and the AI turn starts from a settled board (R283)".
//   Base:    "When the opponent declares an attack that would be lethal to your hero: cancel it,
//            and an AI plays the rest of their turn with random legal actions"
//   Radiant: "When the opponent declares an attack that would be lethal to your hero: cancel it,
//            destroy the attacker, and an AI plays the rest of their turn with random legal
//            actions" — R276 gave it this face, and R283 orders it: the destroy rides the cancel
//            and is collected by a state check before the AI takes the turn. Those cases play real
//            attacks through the harness (the base face's played-out behaviour is proved the same
//            way in my-pawn.test.ts).
//
// WHY THE CONDITION IS TESTED THROUGH `when`. The card's OWN contribution to "would be lethal to
// your hero" — the whole of the M4-T4 row's first clause — is the `when` predicate, a pure function
// of the state and the event, so it is called directly here with a context built the way
// `traps::fire_trap` builds it, which pins each lethal boundary without an AI turn in the way. The
// base face's cancel, lockout and AI turn are played through real attacks in my-pawn.test.ts.
//
// R662's yellow glow (`conditionMet`): on its controller's field while an enemy unit would be lethal
// attacking their hero now, both faces, checked against the attack, at the end of this file.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MY_PAWN: &str = "core-096";
    /// #13 Jlockeed Shredder-10, 8/10 with no keywords: a plain 8-damage swing.
    const ATTACKER: &str = "core-013";
    /// #73 Anti-oneshot Armor: `staticFlags.antiOneshot`, so §4.4 step 3 clamps to 5 (radiant 3).
    const ANTI_ONESHOT: &str = "core-073";
    /// A 1/1 body to be attacked, so Trample excess has something to spill past (§4.4 step 9).
    const SMALL: &str = "core-t-felinor";

    use crate::js;

    fn events_json(s: &Scenario) -> Vec<Value> {
        s.events().iter().map(|event| js(event)).collect()
    }

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
        }
    }

    /// `when` with the context `traps::fire_trap` would hand it: the trap as `self`, its controller as
    /// the controller, and the event under test. Nothing is mutated — the predicate only reads, so it
    /// reads a copy of the scenario's state.
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

    /// p1 holds the trap; p2 is active and swings at p1's hero for 8.
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

    /// Granting the keyword on a fixture is a test's business, not a card's (CLAUDE.md rule 5): the
    /// test writes the state, as TS wrote through the live instance.
    fn grant_trample(s: &mut Scenario, instance_id: &str) {
        find_instance_mut(s.state_mut(), instance_id)
            .expect("the attacker is on the field")
            .granted_keywords = vec![json_as(json!({ "kind": "Trample" }))];
    }

    mod n96_my_pawn_the_trigger_it_registers {
        use super::*;

        #[test]
        fn s4_2_step_4_it_watches_attackdeclared_and_nothing_else() {
            crate::register_all();
            assert_eq!(trigger("base").on, vec![GameEventType::AttackDeclared]);
        }

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
            // TS: `radiant` is not the same object as `base`; here, its trigger runs its own body.
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
            let s = swing(Swing { health: 8, ..Swing::default() });
            assert!(arms(&s, &attack_declared(&id_of(&s, ATTACKER), "hero-p1", false)));
        }

        #[test]
        fn r44_does_not_fire_when_the_hero_would_survive_by_one() {
            crate::register_all();
            let s = swing(Swing { health: 9, ..Swing::default() });
            assert!(!arms(&s, &attack_declared(&id_of(&s, ATTACKER), "hero-p1", false)));
        }

        #[test]
        fn r44_counts_armor_1_armor_makes_the_same_8_damage_swing_survivable_at_8_health() {
            crate::register_all();
            let armored = swing(Swing { health: 8, armor: Some(1), ..Swing::default() });
            assert!(!arms(&armored, &attack_declared(&id_of(&armored, ATTACKER), "hero-p1", false)));

            // 8 − 1 Armor = 7, which is exactly lethal at 7 (§4.4 step 2, §4.5 step 2).
            let exact = swing(Swing { health: 7, armor: Some(1), ..Swing::default() });
            assert!(arms(&exact, &attack_declared(&id_of(&exact, ATTACKER), "hero-p1", false)));
        }

        #[test]
        fn r44_counts_the_anti_oneshot_cap_an_8_damage_swing_projects_5() {
            crate::register_all();
            let capped = swing(Swing { health: 8, backrow: vec![json!(ANTI_ONESHOT)], ..Swing::default() });
            assert!(!arms(&capped, &attack_declared(&id_of(&capped, ATTACKER), "hero-p1", false)));

            let exact = swing(Swing { health: 5, backrow: vec![json!(ANTI_ONESHOT)], ..Swing::default() });
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
            let mut s = swing(Swing { health: 7, field: Some(vec![json!(SMALL)]), ..Swing::default() });
            let attacker = id_of(&s, ATTACKER);
            let victim = id_of(&s, SMALL);

            // §4.4 step 9 sends only the excess on: 8 − the 1/1's 1 health = 7, exactly lethal at 7.
            // Granting the keyword on a fixture is a test's business, not a card's (CLAUDE.md rule 5).
            grant_trample(&mut s, &attacker);
            assert!(arms(&s, &attack_declared(&attacker, &victim, false)));

            let plain = swing(Swing { health: 7, field: Some(vec![json!(SMALL)]), ..Swing::default() });
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
                "p1": { "health": 8, "backrow": [MY_PAWN], "field": [ATTACKER] },
                "p2": { "health": 8 },
            }));
            // p1's own 8-damage swing would be lethal to p2, and p1's trap is not interested.
            assert!(!arms(&s, &attack_declared(&id_of(&s, ATTACKER), "hero-p2", false)));
        }

        #[test]
        fn r121_a_forced_attack_is_declared_by_the_effect_not_the_player_so_a_trigger_keyed_to_an_opponent_s_declaration_does_not_arm() {
            crate::register_all();
            let s = swing(Swing { health: 8, ..Swing::default() });
            // §4.2's last paragraph and R53: a forced attack skips steps 1 to 3 and spends no exertion,
            // and it can happen on the trap owner's own turn, where "an AI plays the rest of their turn"
            // names nobody. `forced` is on the event for exactly this distinction.
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
                // p2's Trample attacker aimed at a unit of p2's OWN: §4.4 step 9 sends the excess to "the
                // target's controller's hero", which is p2's, so p1's trap has nothing to answer.
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

    // The body of the base trap — R44's cancel with the exertion spent, R84's policy playing the rest
    // of the turn, R152's lockout and the trap going to the graveyard once the turn it gave has ended —
    // is proved where its machinery is: crates/engine/tests/rules/effects_combat.rs and
    // crates/cards/tests/cross/my_pawn.rs, and spec 07 in a browser. The radiant face's addition is
    // proved below.

    // -------------------------------------------------------------------------------------------
    // Radiant: "cancel it, destroy the attacker, and an AI plays the rest of their turn" (R283)
    // -------------------------------------------------------------------------------------------

    /// #68, a 5/5 with nothing that fires in combat: the plain lethal swing at a 5-health hero.
    const SORCERER: &str = "core-068";
    /// #56 Jilliax: its radiant face is a 6/4 with Rush, Taunt, Lifesteal, Divine Shield and Reborn.
    const JILLIAX: &str = "core-056";
    /// #66 The Rock, 10/10 Indestructible: the one Indestructible unit in Core since patch v0.1.1.
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
            .unwrap_or_else(|| panic!("no attack was cancelled"));
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

            // Cancelled: no combat, so the hero took nothing (R44).
            s.expect_health(P2, 5);
            assert_eq!(events_json(&s).iter().filter(|event| event["type"] == "attackCancelled").count(), 1);
            assert!(!any_event(&s, "damage"));

            // The attacker is destroyed, after the cancel, and collected right then — before any event
            // of the AI's turn.
            assert_eq!(from_cancel(&s, 3), vec!["attackCancelled", "destroyed", "enteredGraveyard"]);
            assert_eq!(events_on(&s, &attacker.id, "destroyed"), 1);
            s.expect_in_zone(&attacker, "graveyard");

            // …and the AI still plays out the rest of the turn.
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
        fn r283_r46_an_indestructible_attacker_is_knocked_down_instead_and_the_attack_is_cancelled_either_way() {
            crate::register_all();
            // #66 The Rock: a 10/10 Indestructible, in Attack Position, with no Taunt (R347 takes any).
            let (mut s, attacker) = pawn_game(json!(ROCK), 10, "radiant");

            // Cancelled: the 10 never landed.
            s.expect_health(P2, 10);
            assert!(!events_json(&s)
                .iter()
                .any(|event| event["type"] == "damage" && event["sourceId"] == attacker.id.as_str()));

            // R46: the mark does not kill it. It is in Attack Position already and has no Taunt to lose,
            // so the knock-down changes nothing a view shows and reports nothing (R91).
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
            // §6.1: Divine Shield negates the first damage instance; a destroy is a mark, not damage
            // (§6.3), so the shield has nothing to negate. A radiant #20 has the shield and no Reborn, so
            // the destroy is the whole story.
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

            // Cancelled: the 14 never landed.
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
            // #89 in p1's hand, with the mana to play it: the AI may well play it, and R212 would have a
            // card that has moved since the death answer nothing — so the death has to be dispatched, and
            // the Eater's hand trigger run, before the AI's first action.
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
            let cancelled_at = index_of("attackCancelled");
            let ai_acts = all
                .iter()
                .enumerate()
                .position(|(at, event)| {
                    (at as isize) > cancelled_at && (event["type"] == "cardPlayed" || event["type"] == "turnEnded")
                })
                .map_or(-1, |at| at as isize);
            assert!(fed > index_of("destroyed"));
            assert!(fed < ai_acts);
            turn_went_on(&s, P1);
        }

        #[test]
        fn r283_r174_a_radiant_my_pawn_fused_onto_a_radiant_my_pawn_destroys_the_attacker_once_not_its_reborn_body_again() {
            crate::register_all();
            // p2's My Pawn, set, is fused by p1's #85 onto p1's Radiant My Pawn (R61, R77): one trap with
            // both texts, whose second half runs after the first has played p2's turn out (R102).
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

        /// R662 `${face}`: at 7 health it glows for its controller only, and the swing is cancelled.
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

        /// R662 `${face}`: at 8 health it does not glow, and the swing lands.
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

        /// R662 `${face}`: on its controller's own turn it glows for the blow on the board.
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
        fn r662_base_at_7_health_it_glows_for_its_controller_only_and_the_swing_is_cancelled() {
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
        fn r662_radiant_at_7_health_it_glows_for_its_controller_only_and_the_swing_is_cancelled() {
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
