// B5 E5 replacement windows, with E8's heal conversion and E9's redirects (docs/classic-sets.md B5;
// SPEC §4.2, §4.4, §4.5, §3.2; R460, R461, R462, R463). Each moment is proved through a fixture card
// shaped like the Classic or Classic+ card that uses it (`fixtures/damage-combat.ts`): a Final
// Gambit, a Shadowstep, a Blood Moon, a Voidwalker, a Second Wind, a Pile On and a Joro.
//
// Every scenario that runs through `reduce` is also replayed from its start state (§9.3); the ones
// that pause survive `JSON.parse(JSON.stringify(state))` and finish from the round-tripped copy; and
// the moments that read a hidden card (a face-down Trap, a hand) are checked from the other seat's
// view (R97, R177): a replacement that declines leaves that view exactly as a card without one would.
//
// Port of `packages/engine/test/replacements.test.ts`.

use jackioh_engine::effects::{discard, heal};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::damage_combat::{
    anime_armor, answer, argus, blood_moon, echo_gambit, event_types, gambit, gambit_asker, grunt, in_pile,
    joro, leech, mend, notes, phoenix, pile_on, playing, rattle, recorder, replays_to, round_trip,
    second_wind, shadowstep, storm, vital_kill, voidwalker, wall, watcher,
};
use crate::rules::fixtures::harness::{events_of_type, in_hand, put, slot};
use crate::rules::fixtures::scripts::anti_oneshot;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

fn hero(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].hero.health
}

fn backrow_at(state: &GameState, player: PlayerId, lane: usize) -> Option<&CardInstance> {
    state.players[player].backrow.get(lane - 1).and_then(Option::as_ref)
}

fn unit_at(state: &GameState, player: PlayerId, lane: usize) -> Option<&CardInstance> {
    state.players[player]
        .units
        .get(lane - 1)
        .and_then(Option::as_ref)
        .and_then(|pile| pile.first())
}

/// TS `sinkFor(state)`'s three parts side by side, so the state stays readable between engine calls
/// (TS read the same object through `state` and `sink.state`).
struct Bench {
    state: GameState,
    events: Vec<GameEvent>,
    rng: Rng,
}

impl Bench {
    fn sink_for(state: GameState) -> Bench {
        let rng = Rng::new(&state.seed, state.rng_cursor);
        Bench {
            state,
            events: Vec::new(),
            rng,
        }
    }

    fn sink(&mut self) -> EngineSink<'_> {
        EngineSink::new(&mut self.state, &mut self.events, &mut self.rng)
    }

    /// `applyEffects(effects, makeContext(sink, self, options))`, `self` read as it stands now.
    fn apply(&mut self, me: Option<&CardInstance>, options: HookOptions, effects: Vec<Effect>) {
        let me = me.map(|card| find_instance(&self.state, &card.id).cloned().unwrap_or_else(|| card.clone()));
        let mut ctx = make_context(self.sink(), me, options);
        apply_effects(&effects, &mut ctx);
    }

    /// `dealDamage(sink, { source, target, amount })`, each card read as it stands now.
    fn deal(&mut self, source: Option<&CardInstance>, target: DamageTarget, amount: i32) -> i32 {
        let source = source.map(|card| live(&self.state, card).clone());
        deal_damage(
            &mut self.sink(),
            DamageArgs {
                source,
                target,
                amount,
                flags: None,
            },
        )
    }
}

/// The card as it stands in the state now (TS held the live object).
fn live<'a>(state: &'a GameState, card: &CardInstance) -> &'a CardInstance {
    find_instance(state, &card.id).expect("the card is still in the game")
}

fn live_mut<'a>(state: &'a mut GameState, card: &CardInstance) -> &'a mut CardInstance {
    find_instance_mut(state, &card.id).expect("the card is still in the game")
}

fn hero_target(player: PlayerId) -> DamageTarget {
    DamageTarget::Hero { player }
}

fn input(body: Value) -> ActionInput {
    json_as(body)
}

fn of_type(events: &[GameEvent], kind: GameEventType) -> Vec<Value> {
    events_of_type(events, kind)
        .into_iter()
        .map(|event| serde_json::to_value(event).unwrap())
        .collect()
}

fn field_of(events: &[GameEvent], kind: GameEventType, field: &str) -> Vec<Value> {
    of_type(events, kind)
        .into_iter()
        .map(|event| event[field].clone())
        .collect()
}

fn view_json(state: &GameState, viewer: PlayerId) -> Value {
    serde_json::to_value(view_for(state, viewer)).unwrap()
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

fn first_in_hand(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    in_hand(state, def_id, player, None).remove(0)
}

// ---------------------------------------------------------------------------
// §4.4: would take lethal damage (Classic #52 Final Gambit)
// ---------------------------------------------------------------------------

mod e5_would_take_lethal_damage_e9_damage_redirect {
    use super::*;

    #[test]
    fn r460_a_lethal_hit_is_redirected_once_the_first_final_gambit_fires_and_the_second_stays_set() {
        let mut state = playing("dc-gambit-two");
        let attacker = put(&mut state, &grunt().id, slot(P1, Row::Units, 1), Default::default());
        let first = put(&mut state, &gambit().id, slot(P2, Row::Backrow, 1), Default::default());
        let second = put(&mut state, &gambit().id, slot(P2, Row::Backrow, 2), Default::default());
        state.players.p2.hero.health = 2;
        let hand_before = state.players.p2.hand.len();
        let mut game = recorder(state);

        let result = game.play(input(
            json!({ "type": "attack", "attackerId": attacker.id, "targetId": "hero-p2", "playerId": "p1" }),
        ));
        let after = game.state().clone();

        // The hit moved to p1's hero, as a new instance from the same source (E9).
        assert_eq!(hero(&after, P1), 30 - 2);
        // p2 took nothing, then its follow-up healed 10 and drew 3 once the combat was done.
        assert_eq!(hero(&after, P2), 2 + 10);
        assert_eq!(after.players.p2.hand.len(), hand_before + 3);
        assert_eq!(notes(&after), strings(&["gambit:after:p1"]));
        assert_eq!(
            of_type(&result.events, GameEventType::Redirected),
            vec![json!({ "type": "redirected", "what": "damage", "fromId": "hero-p2", "toId": "hero-p1", "byInstanceId": first.id })]
        );
        assert_eq!(
            field_of(&result.events, GameEventType::TrapFired, "instanceId"),
            vec![json!(first.id)]
        );
        // The first is spent to its owner's graveyard; the second re-checked the changed hit and stays set.
        assert!(in_pile(&after, P2, OffFieldZone::Graveyard, &first.id));
        assert_eq!(backrow_at(&after, P2, 2).map(|c| c.id.clone()), Some(second.id.clone()));
        assert!(backrow_at(&after, P2, 2).and_then(|c| c.face_up).is_none());
        assert!(after.work.is_empty());
        assert!(replays_to(&game.start, &game.log, &after));
    }

    #[test]
    fn r460_a_redirected_hit_meets_the_other_heros_replacements_as_a_new_instance_each_card_once() {
        let mut state = playing("dc-gambit-both");
        let attacker = put(&mut state, &grunt().id, slot(P1, Row::Units, 1), Default::default());
        let theirs = put(&mut state, &gambit().id, slot(P2, Row::Backrow, 1), Default::default());
        let mine = put(&mut state, &gambit().id, slot(P1, Row::Backrow, 1), Default::default());
        state.players.p1.hero.health = 1;
        state.players.p2.hero.health = 2;
        let mut game = recorder(state);

        let result = game.play(input(
            json!({ "type": "attack", "attackerId": attacker.id, "targetId": "hero-p2", "playerId": "p1" }),
        ));
        let after = game.state().clone();

        // p2's Gambit sent it to p1, p1's sent it back, and p2 has no second one: p2 falls, p1 wins.
        let moves: Vec<Value> = of_type(&result.events, GameEventType::Redirected)
            .into_iter()
            .map(|e| json!([e["fromId"], e["toId"], e["byInstanceId"]]))
            .collect();
        assert_eq!(
            moves,
            vec![json!(["hero-p2", "hero-p1", theirs.id]), json!(["hero-p1", "hero-p2", mine.id])]
        );
        assert_eq!(hero(&after, P2), 0);
        assert_eq!(hero(&after, P1), 1);
        assert_eq!(
            serde_json::to_value(after.result).unwrap(),
            json!({ "winner": "p1", "reason": "hero-death" })
        );
        // The game ended at the check after the combat, so neither follow-up resolved.
        assert!(notes(&after).is_empty());
        assert!(replays_to(&game.start, &game.log, &after));
    }

    #[test]
    fn two_field_traps_that_redirect_for_ever_stop_at_damage_redirect_cap_and_the_hit_lands_where_it_stands() {
        let mut state = playing("dc-gambit-loop");
        put(&mut state, &echo_gambit().id, slot(P1, Row::Backrow, 1), Default::default());
        put(&mut state, &echo_gambit().id, slot(P2, Row::Backrow, 1), Default::default());
        state.players.p1.hero.health = 1;
        state.players.p2.hero.health = 1;
        let mut b = Bench::sink_for(state);
        b.deal(None, hero_target(P2), 5);
        assert_eq!(
            of_type(&b.events, GameEventType::Redirected).len(),
            DAMAGE_REDIRECT_CAP as usize
        );
        // An even number of moves brings it back to p2, where it lands.
        assert_eq!(DAMAGE_REDIRECT_CAP % 2, 0);
        assert_eq!(hero(&b.state, P2), -4);
        assert_eq!(hero(&b.state, P1), 1);
    }

    #[test]
    fn a_hit_that_is_not_lethal_and_losing_health_leave_a_final_gambit_set() {
        let mut state = playing("dc-gambit-quiet");
        let trap = put(&mut state, &gambit().id, slot(P2, Row::Backrow, 1), Default::default());
        state.players.p2.hero.health = 5;
        let mut b = Bench::sink_for(state);
        b.deal(None, hero_target(P2), 4);
        assert_eq!(hero(&b.state, P2), 1);
        // R18: lose health is not damage, so no replacement answers it, lethal or not.
        lose_health(&mut b.sink(), P2, 3);
        assert_eq!(hero(&b.state, P2), -2);
        assert_eq!(backrow_at(&b.state, P2, 1).map(|c| c.id.clone()), Some(trap.id.clone()));
        assert!(!event_types(&b.events).contains(&"trapFired".to_string()));
    }

    #[test]
    fn fatigue_is_damage_a_lethal_fatigue_draw_is_redirected_and_the_follow_up_waits_for_the_draw_to_end() {
        let mut state = playing("dc-gambit-fatigue");
        put(&mut state, &gambit().id, slot(P2, Row::Backrow, 1), Default::default());
        state.players.p2.library = Vec::new();
        state.players.p2.hero.health = 1;
        let mut b = Bench::sink_for(state);
        draw_one(&mut b.sink(), P2, None);
        assert_eq!(hero(&b.state, P2), 1);
        assert_eq!(hero(&b.state, P1), 29);
        // The follow-up is owed on `state.work` (R113, R462) and the resolution loop resolves it.
        let owed: Vec<String> = b.state.work.iter().map(|item| item.resume.step.clone()).collect();
        assert_eq!(owed, strings(&["after"]));
        settle(&mut b.sink(), Default::default());
        // Heal 10, then three more fatigue draws of 2, 3 and 4.
        assert_eq!(hero(&b.state, P2), 1 + 10 - 2 - 3 - 4);
        assert_eq!(b.state.players.p2.fatigue_count, 4);
        assert!(b.state.work.is_empty());
    }

    #[test]
    fn e9_the_redirected_hit_goes_through_the_other_heros_armor_divisor_and_cap() {
        let mut state = playing("dc-gambit-guarded");
        put(&mut state, &gambit().id, slot(P2, Row::Backrow, 1), Default::default());
        put(&mut state, &argus().id, slot(P1, Row::Backrow, 1), Default::default());
        put(&mut state, &anti_oneshot().id, slot(P1, Row::Backrow, 2), Default::default());
        state.players.p1.hero.armor = 1;
        state.players.p2.hero.health = 9;
        let mut b = Bench::sink_for(state);
        let dealt = b.deal(None, hero_target(P2), 12);
        // 12 − 1 Armor = 11, halved and rounded up = 6, capped at Anti-oneshot's 5.
        assert_eq!(dealt, 5);
        assert_eq!(hero(&b.state, P1), 25);
        assert_eq!(hero(&b.state, P2), 9);
        // And a cap alone keeps a hit from being lethal in the first place.
        put(&mut b.state, &anime_armor().id, slot(P2, Row::Units, 1), Default::default());
        b.state.players.p2.hero.health = 2;
        let second = put(&mut b.state, &gambit().id, slot(P2, Row::Backrow, 3), Default::default());
        b.deal(None, hero_target(P2), 12);
        assert_eq!(hero(&b.state, P2), 1);
        assert_eq!(backrow_at(&b.state, P2, 3).map(|c| c.id.clone()), Some(second.id.clone()));
    }

    #[test]
    fn r113_a_follow_up_that_asks_pauses_after_the_replaced_event_survives_a_round_trip_and_replays() {
        let mut state = playing("dc-gambit-pause");
        let attacker = put(&mut state, &grunt().id, slot(P1, Row::Units, 1), Default::default());
        put(&mut state, &gambit_asker().id, slot(P2, Row::Backrow, 1), Default::default());
        state.players.p2.hero.health = 2;
        let mut game = recorder(state);

        game.play(input(
            json!({ "type": "attack", "attackerId": attacker.id, "targetId": "hero-p2", "playerId": "p1" }),
        ));
        let paused = game.state().clone();
        assert_eq!(hero(&paused, P1), 28);
        assert_eq!(notes(&paused), strings(&["asker:before"]));
        assert_eq!(paused.pending.as_ref().map(|p| p.player_id), Some(P2));
        // The rest of the follow-up is owed as plain data.
        let work_again: Vec<WorkItem> =
            serde_json::from_value(serde_json::to_value(&paused.work).unwrap()).unwrap();
        assert_eq!(work_again, paused.work);

        let resumed = answer(&round_trip(&paused)).state;
        assert_eq!(notes(&resumed), strings(&["asker:before", "asker:answered", "asker:tail"]));
        assert_eq!(hero(&resumed, P2), 12);
        assert!(resumed.work.is_empty());
        assert!(resumed.pending.is_none());

        // The live game answered the same prompt: its log replays to the same state.
        let pending = paused.pending.clone().expect("expected a prompt");
        game.play(input(json!({
            "type": "answer", "choiceId": pending.id, "selection": [{ "pick": "none" }], "playerId": "p2"
        })));
        assert_eq!(notes(game.state()), notes(&resumed));
        assert!(replays_to(&game.start, &game.log, game.state()));
    }

    #[test]
    fn r177_a_face_down_final_gambit_that_declines_a_hit_tells_the_other_seat_nothing() {
        let mut with_gambit = playing("dc-gambit-hidden");
        let mut with_other = playing("dc-gambit-hidden");
        let a = put(&mut with_gambit, &grunt().id, slot(P1, Row::Units, 1), Default::default());
        put(&mut with_gambit, &gambit().id, slot(P2, Row::Backrow, 1), Default::default());
        let b = put(&mut with_other, &grunt().id, slot(P1, Row::Units, 1), Default::default());
        put(&mut with_other, &shadowstep().id, slot(P2, Row::Backrow, 1), Default::default());
        let mut one = recorder(with_gambit);
        let mut two = recorder(with_other);
        one.play(input(json!({ "type": "attack", "attackerId": a.id, "targetId": "hero-p2", "playerId": "p1" })));
        two.play(input(json!({ "type": "attack", "attackerId": b.id, "targetId": "hero-p2", "playerId": "p1" })));
        assert_eq!(view_json(one.state(), P1), view_json(two.state(), P1));
    }
}

// ---------------------------------------------------------------------------
// Heals: would be healed, and heal into damage (Classic+ #22 Blood Moon)
// ---------------------------------------------------------------------------

mod e5_would_be_healed_e8_heal_becomes_damage {
    use super::*;

    #[test]
    fn r462_an_enemys_heal_sets_blood_moon_off_and_is_converted_and_the_rest_of_the_turn_converts_too() {
        let mut state = playing("dc-moon");
        let moon = put(&mut state, &blood_moon().id, slot(P2, Row::Backrow, 1), Default::default());
        let first_mend = first_in_hand(&mut state, &mend().id, P1);
        let second_mend = first_in_hand(&mut state, &mend().id, P1);
        let third_mend = first_in_hand(&mut state, &mend().id, P1);
        state.players.p1.hero.armor = 3;
        let mut game = recorder(state);

        let first = game.play(input(json!({
            "type": "play", "instanceId": first_mend.id, "targets": [{ "pick": "hero", "player": "p1" }], "playerId": "p1"
        })));
        let now = game.state().clone();
        // Pierce: the hero's Armor 3 stops none of the 5, and the damage is Blood Moon's.
        assert_eq!(hero(&now, P1), 25);
        assert_eq!(
            of_type(&first.events, GameEventType::Damage),
            vec![json!({ "type": "damage", "sourceId": moon.id, "targetId": "hero-p1", "amount": 5, "combat": false })]
        );
        assert!(of_type(&first.events, GameEventType::Healed).is_empty());
        assert!(in_pile(&now, P2, OffFieldZone::Graveyard, &moon.id));
        let kinds: Vec<Value> = now
            .players
            .p2
            .mods
            .iter()
            .map(|modifier| serde_json::to_value(modifier).unwrap()["kind"].clone())
            .collect();
        assert_eq!(kinds, vec![json!("healToDamage")]);

        // The modifier converts the next heal on p1 with no trap left to fire.
        let second = game.play(input(json!({
            "type": "play", "instanceId": second_mend.id, "targets": [{ "pick": "hero", "player": "p1" }], "playerId": "p1"
        })));
        let now = game.state().clone();
        assert_eq!(hero(&now, P1), 20);
        assert!(!event_types(&second.events).contains(&"trapFired".to_string()));

        // A heal on Blood Moon's own side is no enemy's: it heals.
        game.play(input(json!({
            "type": "play", "instanceId": third_mend.id, "targets": [{ "pick": "hero", "player": "p2" }], "playerId": "p1"
        })));
        let now = game.state().clone();
        assert_eq!(hero(&now, P2), 35);
        assert!(replays_to(&game.start, &game.log, &now));

        // "For the rest of this turn": gone at the turn's cleanup.
        game.play(input(json!({ "type": "endTurn", "playerId": "p1" })));
        let now = game.state().clone();
        assert!(now.players.p2.mods.is_empty());
        let mut b = Bench::sink_for(now);
        heal_hero(&mut b.sink(), P1, 4);
        assert_eq!(hero(&b.state, P1), 24);
    }

    #[test]
    fn r462_lifesteal_heal_up_to_and_heal_to_full_are_heals_a_heal_on_an_undamaged_unit_converts_its_stated_amount() {
        let mut state = playing("dc-moon-kinds");
        put(&mut state, &blood_moon().id, slot(P2, Row::Backrow, 1), Default::default());
        let sucker = put(&mut state, &leech().id, slot(P1, Row::Units, 1), Default::default());
        let target = put(&mut state, &wall().id, slot(P2, Row::Units, 1), Default::default());
        let mut b = Bench::sink_for(state);
        // Lifesteal: 3 dealt heals p1 3, which Blood Moon turns into 3 Pierce damage on p1.
        let hit = DamageTarget::Unit {
            instance: live(&b.state, &target).clone(),
        };
        b.deal(Some(&sucker), hit, 3);
        assert_eq!(hero(&b.state, P1), 27);
        // Heal up to 30: the 3 it would restore.
        b.apply(
            Some(&sucker),
            HookOptions::default(),
            vec![heal(json_as(json!({ "target": { "of": "selfHero" }, "upTo": 30 })))],
        );
        assert_eq!(hero(&b.state, P1), 24);
        // Heal to full on the leech's 2 damage: 2.
        live_mut(&mut b.state, &sucker).damage = 2;
        b.apply(
            Some(&sucker),
            HookOptions::default(),
            vec![heal(json_as(json!({ "target": { "of": "self" }, "toFull": true })))],
        );
        assert_eq!(live(&b.state, &sucker).damage, 4);
        // Heal 5 on an undamaged unit: 5, though it would restore nothing.
        let fresh = put(&mut b.state, &grunt().id, slot(P1, Row::Units, 2), Default::default());
        b.apply(
            Some(&sucker),
            HookOptions::default(),
            vec![heal(json_as(
                json!({ "target": { "of": "instance", "instanceId": fresh.id }, "amount": 5 }),
            ))],
        );
        assert_eq!(live(&b.state, &fresh).damage, 5);
        // A heal of nothing (heal to full on an undamaged unit) is no heal at all: nothing converts.
        let whole = put(&mut b.state, &grunt().id, slot(P1, Row::Units, 3), Default::default());
        let before = b.events.len();
        b.apply(
            Some(&sucker),
            HookOptions::default(),
            vec![heal(json_as(
                json!({ "target": { "of": "instance", "instanceId": whole.id }, "toFull": true }),
            ))],
        );
        assert_eq!(b.events.len(), before);
        assert_eq!(live(&b.state, &whole).damage, 0);
    }

    #[test]
    fn e7_set_health_is_no_heal_blood_moon_stays_set() {
        let mut state = playing("dc-moon-set");
        let moon = put(&mut state, &blood_moon().id, slot(P2, Row::Backrow, 1), Default::default());
        state.players.p1.hero.health = 5;
        let spell = first_in_hand(&mut state, &vital_kill().id, P1);
        let mut game = recorder(state);
        let result = game.play(input(json!({
            "type": "play", "instanceId": spell.id, "targets": [{ "pick": "hero", "player": "p1" }], "playerId": "p1"
        })));
        assert_eq!(hero(game.state(), P1), 13);
        assert_eq!(
            of_type(&result.events, GameEventType::HealthSet),
            vec![json!({ "type": "healthSet", "player": "p1", "health": 13, "sourceId": spell.id })]
        );
        assert_eq!(
            backrow_at(game.state(), P2, 1).map(|c| c.id.clone()),
            Some(moon.id.clone())
        );
        assert!(replays_to(&game.start, &game.log, game.state()));
    }

    #[test]
    fn e8_the_radiant_field_trap_fires_once_stays_face_up_and_converts_from_then_on() {
        let mut state = playing("dc-moon-radiant");
        let moon = put(
            &mut state,
            &blood_moon().id,
            slot(P2, Row::Backrow, 1),
            json_as(json!({ "radiant": true })),
        );
        let mut b = Bench::sink_for(state);
        heal_hero(&mut b.sink(), P1, 4);
        assert_eq!(hero(&b.state, P1), 26);
        assert_eq!(
            field_of(&b.events, GameEventType::TrapFired, "instanceId"),
            vec![json!(moon.id)]
        );
        assert_eq!(backrow_at(&b.state, P2, 1).and_then(|c| c.face_up), Some(true));
        // From now on: the face-up Field Trap converts by its text, with no firing and no modifier.
        heal_hero(&mut b.sink(), P1, 6);
        assert_eq!(hero(&b.state, P1), 20);
        assert_eq!(of_type(&b.events, GameEventType::TrapFired).len(), 1);
        assert!(b.state.players.p2.mods.is_empty());
        // Its own side's heals go through.
        heal_hero(&mut b.sink(), P2, 6);
        assert_eq!(hero(&b.state, P2), 36);
    }
}

// ---------------------------------------------------------------------------
// §4.5 step 1: would die (Classic #14's Radiant face)
// ---------------------------------------------------------------------------

mod e5_would_die {
    use super::*;

    #[test]
    fn r460_its_controllers_dying_units_flicker_instead_one_firing_for_them_all_a_second_shadowstep_stays_set() {
        let mut state = playing("dc-shadowstep");
        let mine = vec![
            put(&mut state, &rattle().id, slot(P1, Row::Units, 1), Default::default()),
            put(&mut state, &grunt().id, slot(P1, Row::Units, 2), Default::default()),
        ];
        let theirs = put(&mut state, &rattle().id, slot(P2, Row::Units, 1), Default::default());
        let trap = put(&mut state, &shadowstep().id, slot(P1, Row::Backrow, 1), Default::default());
        let spare = put(&mut state, &shadowstep().id, slot(P1, Row::Backrow, 2), Default::default());
        for unit in &mine {
            live_mut(&mut state, unit).buffs = AttackHealth { attack: 1, health: 0 };
        }
        let spell = first_in_hand(&mut state, &storm().id, P1);
        let mut game = recorder(state);

        let result = game.play(input(json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" })));
        let after = game.state().clone();

        // Back in their zones at once, reset (no buff, no damage) and summoning sick.
        for (lane, unit) in mine.iter().enumerate() {
            let back = unit_at(&after, P1, lane + 1).expect("back in its zone");
            assert_eq!(back.id, unit.id);
            assert_eq!(back.damage, 0);
            assert_eq!(back.buffs, AttackHealth { attack: 0, health: 0 });
            assert_eq!(back.summoned_turn, Some(after.turn));
        }
        let mine_ids: Vec<Value> = mine.iter().map(|u| json!(u.id)).collect();
        assert_eq!(field_of(&result.events, GameEventType::Flickered, "instanceId"), mine_ids);
        // No death for them: no `destroyed`, no Death hook; the enemy's rattle dies as ever.
        assert_eq!(
            field_of(&result.events, GameEventType::Destroyed, "instanceId"),
            vec![json!(theirs.id)]
        );
        let joined = mine.iter().map(|u| u.id.clone()).collect::<Vec<_>>().join(",");
        assert_eq!(
            notes(&after),
            vec!["rattle:death:p2".to_string(), format!("shadowstep:{joined}")]
        );
        assert_eq!(
            field_of(&result.events, GameEventType::TrapFired, "instanceId"),
            vec![json!(trap.id)]
        );
        assert_eq!(backrow_at(&after, P1, 2).map(|c| c.id.clone()), Some(spare.id.clone()));
        assert_eq!(after.counters.destroyed, 1);
        assert!(replays_to(&game.start, &game.log, &after));
    }

    #[test]
    fn r462_a_sacrifice_is_not_offered_the_would_die_window() {
        let mut state = playing("dc-shadowstep-sacrifice");
        let unit = put(&mut state, &rattle().id, slot(P1, Row::Units, 1), Default::default());
        let trap = put(&mut state, &shadowstep().id, slot(P1, Row::Backrow, 1), Default::default());
        let mut b = Bench::sink_for(state);
        let victim = live(&b.state, &unit).clone();
        sacrifice_now(&mut b.sink(), &victim);
        assert!(in_pile(&b.state, P1, OffFieldZone::Graveyard, &unit.id));
        assert_eq!(notes(&b.state), strings(&["rattle:death:p1"]));
        assert_eq!(backrow_at(&b.state, P1, 1).map(|c| c.id.clone()), Some(trap.id.clone()));
    }

    #[test]
    fn the_follow_up_reads_what_it_flickered_from_its_record() {
        let mut state = playing("dc-shadowstep-record");
        let unit = put(&mut state, &grunt().id, slot(P1, Row::Units, 3), Default::default());
        put(&mut state, &shadowstep().id, slot(P1, Row::Backrow, 1), Default::default());
        live_mut(&mut state, &unit).marked_destroyed = Some(true);
        let mut b = Bench::sink_for(state);
        state_check(&mut b.sink());
        let record = b
            .state
            .work
            .first()
            .and_then(|owed| replacement_of(&owed.resume.data))
            .expect("a replacement record");
        assert_eq!(
            serde_json::to_value(&record.flickered).unwrap(),
            json!([{ "instanceId": unit.id, "defId": grunt().id, "radiant": false }])
        );
        assert_eq!(
            serde_json::to_value(&record.event).unwrap(),
            json!({ "moment": "wouldDie", "units": [{ "instanceId": unit.id, "controller": "p1" }] })
        );
    }
}

// ---------------------------------------------------------------------------
// Every move into a graveyard (Classic #50, #28, #60)
// ---------------------------------------------------------------------------

mod e5_would_go_to_a_graveyard {
    use super::*;

    #[test]
    fn r461_a_unit_exiled_instead_of_dying_has_not_died_no_death_no_reborn_no_destroyed_count() {
        let mut state = playing("dc-void");
        put(&mut state, &voidwalker().id, slot(P1, Row::Units, 1), Default::default());
        let bird = put(&mut state, &phoenix().id, slot(P2, Row::Units, 1), Default::default());
        let bell = put(&mut state, &rattle().id, slot(P1, Row::Units, 2), Default::default());
        let spell = first_in_hand(&mut state, &storm().id, P1);
        let exiled_before = state.counters.exiled;
        let mut game = recorder(state);

        let result = game.play(input(json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" })));
        let after = game.state().clone();

        assert!(in_pile(&after, P2, OffFieldZone::Exile, &bird.id));
        assert!(in_pile(&after, P1, OffFieldZone::Exile, &bell.id));
        assert!(unit_at(&after, P2, 1).is_none());
        assert!(after.reserved.is_empty());
        assert!(of_type(&result.events, GameEventType::Destroyed).is_empty());
        assert!(notes(&after).is_empty());
        assert_eq!(after.counters.destroyed, 0);
        // The Spell itself, on its way to its graveyard, is exiled too; three exiles in all.
        assert!(in_pile(&after, P1, OffFieldZone::Exile, &spell.id));
        assert_eq!(after.counters.exiled, exiled_before + 3);
        let mut exiled: Vec<String> = field_of(&result.events, GameEventType::Exiled, "instanceId")
            .into_iter()
            .map(|id| id.as_str().unwrap_or_default().to_string())
            .collect();
        exiled.sort();
        let mut expected = vec![bird.id.clone(), bell.id.clone(), spell.id.clone()];
        expected.sort();
        assert_eq!(exiled, expected);
        assert!(of_type(&result.events, GameEventType::EnteredGraveyard).is_empty());
        assert!(replays_to(&game.start, &game.log, &after));
    }

    #[test]
    fn r463_a_voidwalkers_own_card_reaches_its_graveyard_and_so_does_every_card_that_dies_with_it() {
        let mut state = playing("dc-void-together");
        let walker = put(&mut state, &voidwalker().id, slot(P1, Row::Units, 1), Default::default());
        let bell = put(&mut state, &rattle().id, slot(P2, Row::Units, 1), Default::default());
        live_mut(&mut state, &walker).marked_destroyed = Some(true);
        live_mut(&mut state, &bell).marked_destroyed = Some(true);
        let mut b = Bench::sink_for(state);
        state_check(&mut b.sink());
        assert!(in_pile(&b.state, P1, OffFieldZone::Graveyard, &walker.id));
        assert!(in_pile(&b.state, P2, OffFieldZone::Graveyard, &bell.id));
        assert_eq!(notes(&b.state), strings(&["rattle:death:p2"]));
    }

    #[test]
    fn the_radiant_voidwalker_exiles_only_its_opponents_cards_a_discard_is_replaced_like_any_move() {
        let mut state = playing("dc-void-radiant");
        put(
            &mut state,
            &voidwalker().id,
            slot(P1, Row::Units, 1),
            json_as(json!({ "radiant": true })),
        );
        let own = put(&mut state, &rattle().id, slot(P1, Row::Units, 2), Default::default());
        let foe = put(&mut state, &rattle().id, slot(P2, Row::Units, 2), Default::default());
        live_mut(&mut state, &own).marked_destroyed = Some(true);
        live_mut(&mut state, &foe).marked_destroyed = Some(true);
        let mut b = Bench::sink_for(state);
        state_check(&mut b.sink());
        assert!(in_pile(&b.state, P1, OffFieldZone::Graveyard, &own.id));
        assert!(in_pile(&b.state, P2, OffFieldZone::Exile, &foe.id));
        assert_eq!(notes(&b.state), strings(&["rattle:death:p1"]));
    }

    #[test]
    fn a_fired_trap_on_its_way_to_its_graveyard_is_exiled_under_a_voidwalker() {
        let mut state = playing("dc-void-trap");
        put(&mut state, &voidwalker().id, slot(P1, Row::Units, 1), Default::default());
        let trap = put(&mut state, &gambit().id, slot(P2, Row::Backrow, 1), Default::default());
        state.players.p2.hero.health = 3;
        let mut b = Bench::sink_for(state);
        b.deal(None, hero_target(P2), 5);
        assert!(in_pile(&b.state, P2, OffFieldZone::Exile, &trap.id));
        assert_eq!(
            field_of(&b.events, GameEventType::Exiled, "instanceId"),
            vec![json!(trap.id)]
        );
    }

    #[test]
    fn second_wind_exiles_its_controllers_own_cards_and_no_one_elses() {
        let mut state = playing("dc-wind");
        put(&mut state, &second_wind().id, slot(P1, Row::Backrow, 1), Default::default());
        let own = put(&mut state, &grunt().id, slot(P1, Row::Units, 1), Default::default());
        let foe = put(&mut state, &grunt().id, slot(P2, Row::Units, 1), Default::default());
        live_mut(&mut state, &own).marked_destroyed = Some(true);
        live_mut(&mut state, &foe).marked_destroyed = Some(true);
        let mut b = Bench::sink_for(state);
        state_check(&mut b.sink());
        assert!(in_pile(&b.state, P1, OffFieldZone::Exile, &own.id));
        assert!(in_pile(&b.state, P2, OffFieldZone::Graveyard, &foe.id));
    }

    #[test]
    fn pile_on_goes_to_the_bottom_of_its_owners_library_when_it_resolves_and_when_it_is_discarded() {
        let mut state = playing("dc-pile");
        let spell = first_in_hand(&mut state, &pile_on().id, P1);
        let mut game = recorder(state);
        let result = game.play(input(json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" })));
        let after = game.state().clone();
        let library = &after.players.p1.library;
        let bottom = library.len() - 1;
        assert_eq!(library.last().map(|c| c.id.clone()), Some(spell.id.clone()));
        assert_eq!(notes(&after), strings(&["pile-on:cry"]));
        let shuffled: Vec<Value> = of_type(&result.events, GameEventType::ShuffledIn)
            .into_iter()
            .map(|e| json!([e["instanceId"], e["position"]]))
            .collect();
        assert_eq!(shuffled, vec![json!([spell.id, bottom])]);
        // R311: it went in openly, so its owner's library list knows it.
        assert_eq!(
            library.last().and_then(|c| c.known_as.clone()),
            Some(KnownAs {
                def_id: pile_on().id,
                radiant: false
            })
        );
        assert!(replays_to(&game.start, &game.log, &after));
        // The other seat is told a card went into p1's library, and not where (R97).
        let seen = view_for(&after, P2)
            .events
            .into_iter()
            .find(|event| event.event_type() == GameEventType::ShuffledIn);
        let seen_json = seen.as_ref().map(|event| serde_json::to_value(event).unwrap());
        assert_eq!(seen_json.as_ref().map(|e| e["type"].clone()), Some(json!("shuffledIn")));
        assert_eq!(seen_json.as_ref().map(|e| e["player"].clone()), Some(json!("p1")));
        let position = match &seen {
            Some(GameEvent::ShuffledIn { position, .. }) => Some(*position),
            _ => None,
        };
        assert_ne!(position, Some(bottom as i32));

        // Discarded from a hand: the same replacement, whatever sends it.
        let mut second = playing("dc-pile-discard");
        let held = first_in_hand(&mut second, &pile_on().id, P1);
        let mut b = Bench::sink_for(second);
        b.apply(
            None,
            HookOptions {
                controller: Some(P1),
                ..Default::default()
            },
            vec![discard(json_as(json!({ "target": { "of": "instance", "instanceId": held.id } })))],
        );
        let lib = &b.state.players.p1.library;
        assert_eq!(lib.last().map(|c| c.id.clone()), Some(held.id.clone()));
        let moves: Vec<String> = event_types(&b.events)
            .into_iter()
            .filter(|kind| kind == "discarded" || kind == "shuffledIn")
            .collect();
        assert_eq!(moves, strings(&["discarded", "shuffledIn"]));
    }

    #[test]
    fn r460_r68_replacements_of_one_move_apply_in_r68s_order_the_active_sides_first_each_once() {
        // p1 active: p1's Voidwalker (a unit) comes before p1's Second Wind (backrow) and before the card
        // itself — exiled, and nothing after it re-checks a card no longer on its way to a graveyard.
        let mut one = playing("dc-order-one");
        put(&mut one, &voidwalker().id, slot(P1, Row::Units, 1), Default::default());
        put(&mut one, &second_wind().id, slot(P1, Row::Backrow, 1), Default::default());
        let first = first_in_hand(&mut one, &pile_on().id, P1);
        let mut a = recorder(one);
        a.play(input(json!({ "type": "play", "instanceId": first.id, "playerId": "p1" })));
        assert!(in_pile(a.state(), P1, OffFieldZone::Exile, &first.id));

        // The opponent's Voidwalker comes after the active side, Pile On's own clause included.
        let mut two = playing("dc-order-two");
        put(&mut two, &voidwalker().id, slot(P2, Row::Units, 1), Default::default());
        let second = first_in_hand(&mut two, &pile_on().id, P1);
        let mut b = recorder(two);
        b.play(input(json!({ "type": "play", "instanceId": second.id, "playerId": "p1" })));
        let library = &b.state().players.p1.library;
        assert_eq!(library.last().map(|c| c.id.clone()), Some(second.id.clone()));

        // Second Wind (backrow) comes before the card itself on the same side.
        let mut three = playing("dc-order-three");
        put(&mut three, &second_wind().id, slot(P1, Row::Backrow, 1), Default::default());
        let third = first_in_hand(&mut three, &pile_on().id, P1);
        let mut c = recorder(three);
        c.play(input(json!({ "type": "play", "instanceId": third.id, "playerId": "p1" })));
        assert!(in_pile(c.state(), P1, OffFieldZone::Exile, &third.id));
    }
}

// ---------------------------------------------------------------------------
// "A friendly unit is targeted" (Classic #33 Joro)
// ---------------------------------------------------------------------------

mod e5_a_friendly_unit_is_targeted_e9_attack_redirect {
    use super::*;

    #[test]
    fn an_attack_on_a_unit_is_moved_to_the_joro_its_controller_summons_from_hand() {
        let mut state = playing("dc-joro");
        let attacker = put(&mut state, &grunt().id, slot(P1, Row::Units, 1), Default::default());
        let chosen = put(&mut state, &wall().id, slot(P2, Row::Units, 1), Default::default());
        let decoy = first_in_hand(&mut state, &joro().id, P2);
        put(&mut state, &watcher().id, slot(P2, Row::Backrow, 1), Default::default());
        let mut game = recorder(state);

        let result = game.play(input(
            json!({ "type": "attack", "attackerId": attacker.id, "targetId": chosen.id, "playerId": "p1" }),
        ));
        let after = game.state().clone();

        let opening: Vec<String> = event_types(&result.events).into_iter().take(3).collect();
        assert_eq!(opening, strings(&["summoned", "redirected", "attackDeclared"]));
        let summoned = of_type(&result.events, GameEventType::Summoned);
        let entry = summoned.first().expect("a summon");
        assert_eq!(entry["instanceId"], json!(decoy.id));
        assert_eq!(entry["player"], json!("p2"));
        assert_eq!(entry["row"], json!("units"));
        assert_eq!(entry["lane"], json!(2));
        assert_eq!(
            of_type(&result.events, GameEventType::Redirected),
            vec![json!({ "type": "redirected", "what": "attack", "fromId": chosen.id, "toId": decoy.id, "byInstanceId": decoy.id })]
        );
        assert_eq!(
            field_of(&result.events, GameEventType::AttackDeclared, "targetId").first(),
            Some(&json!(decoy.id))
        );
        // The 2/2 killed the 1/1 Joro and took 1 back; the wall it chose was never hit.
        assert!(in_pile(&after, P2, OffFieldZone::Graveyard, &decoy.id));
        assert_eq!(find_instance(&after, &chosen.id).map(|c| c.damage), Some(0));
        assert_eq!(find_instance(&after, &attacker.id).map(|c| c.damage), Some(1));
        // The declaration reached the traps once, in its window, not again from the frontier (R100); the
        // interposer's summon reached them in that window too, before the combat.
        assert_eq!(
            notes(&after),
            vec![format!("watch:attack:{}", decoy.id), format!("watch:summon:{}", decoy.id)]
        );
        assert!(replays_to(&game.start, &game.log, &after));
    }

    #[test]
    fn no_open_zone_an_attack_on_the_hero_or_a_forced_attack_joro_stays_in_hand() {
        let mut full = playing("dc-joro-full");
        let attacker = put(&mut full, &grunt().id, slot(P1, Row::Units, 1), Default::default());
        let chosen = put(&mut full, &wall().id, slot(P2, Row::Units, 1), Default::default());
        for lane in [2, 3, 4, 5] {
            put(&mut full, &wall().id, slot(P2, Row::Units, lane), Default::default());
        }
        let decoy = first_in_hand(&mut full, &joro().id, P2);
        let mut one = recorder(full);
        one.play(input(
            json!({ "type": "attack", "attackerId": attacker.id, "targetId": chosen.id, "playerId": "p1" }),
        ));
        assert!(in_pile(one.state(), P2, OffFieldZone::Hand, &decoy.id));
        assert_eq!(find_instance(one.state(), &chosen.id).map(|c| c.damage), Some(2));

        let mut open = playing("dc-joro-hero");
        let striker = put(&mut open, &grunt().id, slot(P1, Row::Units, 1), Default::default());
        put(&mut open, &wall().id, slot(P2, Row::Units, 1), Default::default());
        let held = first_in_hand(&mut open, &joro().id, P2);
        let mut two = recorder(open);
        two.play(input(
            json!({ "type": "attack", "attackerId": striker.id, "targetId": "hero-p2", "playerId": "p1" }),
        ));
        assert!(in_pile(two.state(), P2, OffFieldZone::Hand, &held.id));
    }

    #[test]
    fn the_play_pipelines_half_a_pick_of_a_friendly_unit_is_answered_through_the_same_hook() {
        let mut state = playing("dc-joro-pick");
        let chosen = put(&mut state, &wall().id, slot(P2, Row::Units, 1), Default::default());
        let decoy = first_in_hand(&mut state, &joro().id, P2);
        let mut b = Bench::sink_for(state);
        // Its own controller's pick is no opponent's targeting.
        let target = live(&b.state, &chosen).clone();
        let own = answer_targeting(
            &mut b.sink(),
            AnswerTargetingArgs {
                target: target.clone(),
                by: P2,
                what: TargetedWhat::Target,
            },
        );
        assert!(own.is_none());
        let moved = answer_targeting(
            &mut b.sink(),
            AnswerTargetingArgs {
                target,
                by: P1,
                what: TargetedWhat::Target,
            },
        );
        assert_eq!(moved.map(|card| card.id), Some(decoy.id.clone()));
        assert_eq!(
            of_type(&b.events, GameEventType::Redirected),
            vec![json!({ "type": "redirected", "what": "target", "fromId": chosen.id, "toId": decoy.id, "byInstanceId": decoy.id })]
        );
        assert_eq!(
            unit_at(&b.state, P2, 2).and_then(|c| c.summoned_turn),
            Some(b.state.turn)
        );
    }

    #[test]
    fn r177_a_joro_that_cannot_answer_leaves_the_attackers_view_as_any_other_hand_card_would() {
        let build = |card: &str| -> GameState {
            let mut state = playing("dc-joro-hidden");
            let attacker = put(&mut state, &grunt().id, slot(P1, Row::Units, 1), Default::default());
            put(&mut state, &wall().id, slot(P2, Row::Units, 1), Default::default());
            in_hand(&mut state, card, P2, None);
            let mut game = recorder(state);
            game.play(input(
                json!({ "type": "attack", "attackerId": attacker.id, "targetId": "hero-p2", "playerId": "p1" }),
            ));
            game.state().clone()
        };
        assert_eq!(
            view_json(&build(&joro().id), P1),
            view_json(&build(&grunt().id), P1)
        );
    }
}

// ---------------------------------------------------------------------------
// R97, R177: what each seat reads of the events these moments emit
// ---------------------------------------------------------------------------

mod r97_r177_the_replacement_events_in_both_views {
    use super::*;

    #[test]
    fn a_redirect_a_flicker_and_the_trap_that_did_them_are_public_to_both_seats_once_fired() {
        let mut state = playing("dc-views");
        let attacker = put(&mut state, &grunt().id, slot(P1, Row::Units, 1), Default::default());
        let trap = put(&mut state, &gambit().id, slot(P2, Row::Backrow, 1), Default::default());
        state.players.p2.hero.health = 2;
        let mut game = recorder(state);
        game.play(input(
            json!({ "type": "attack", "attackerId": attacker.id, "targetId": "hero-p2", "playerId": "p1" }),
        ));
        for viewer in [P1, P2] {
            let view = view_for(game.state(), viewer);
            let events = &view.events;
            assert_eq!(
                of_type(events, GameEventType::Redirected),
                vec![json!({ "type": "redirected", "what": "damage", "fromId": "hero-p2", "toId": "hero-p1", "byInstanceId": trap.id })]
            );
            // `trapFired` names the trap to its controller only, as every firing does; the other seat reads
            // the card where it went, its owner's graveyard.
            let fired = if viewer == P2 { gambit().id } else { "hidden".to_string() };
            assert_eq!(
                field_of(events, GameEventType::TrapFired, "defId"),
                vec![json!(fired)]
            );
            let side = if viewer == P2 { &view.you } else { &view.opponent };
            let graveyard: Vec<String> = side.graveyard.iter().map(|card| card.instance_id.clone()).collect();
            assert!(graveyard.contains(&trap.id));
        }

        let mut flick = playing("dc-views-flicker");
        let unit = put(&mut flick, &grunt().id, slot(P1, Row::Units, 1), Default::default());
        put(&mut flick, &shadowstep().id, slot(P1, Row::Backrow, 1), Default::default());
        let spell = first_in_hand(&mut flick, &storm().id, P1);
        let mut other = recorder(flick);
        other.play(input(json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" })));
        for viewer in [P1, P2] {
            let flickered = of_type(&view_for(other.state(), viewer).events, GameEventType::Flickered);
            assert_eq!(
                flickered,
                vec![json!({ "type": "flickered", "player": "p1", "instanceId": unit.id, "defId": grunt().id, "row": "units", "lane": 1 })]
            );
        }
    }

    #[test]
    fn r177_a_face_down_blood_moon_or_shadowstep_that_declines_leaves_the_other_seats_view_as_another_trap_would() {
        let build = |trap_id: &str| -> GameState {
            let mut state = playing("dc-views-decline");
            put(&mut state, trap_id, slot(P1, Row::Backrow, 1), Default::default());
            let foe = put(&mut state, &rattle().id, slot(P2, Row::Units, 1), Default::default());
            live_mut(&mut state, &foe).marked_destroyed = Some(true);
            // TS put the Mend in the recorder's live state (the same object as `state`) right after making
            // the recorder; the recorder's start copy is never read here, so it goes in first.
            // p1 heals its own hero (no enemy of p1's trap) and p2's unit dies (none of p1's units).
            let cure = first_in_hand(&mut state, &mend().id, P1);
            let mut game = recorder(state);
            game.play(input(json!({
                "type": "play", "instanceId": cure.id, "targets": [{ "pick": "hero", "player": "p1" }], "playerId": "p1"
            })));
            game.state().clone()
        };
        let moon = build(&blood_moon().id);
        let step = build(&shadowstep().id);
        let plain = build(&gambit().id);
        assert_eq!(view_json(&moon, P2), view_json(&plain, P2));
        assert_eq!(view_json(&step, P2), view_json(&plain, P2));
    }
}
