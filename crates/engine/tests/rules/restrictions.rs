// B5 E35 unit restrictions and statuses (SPEC §4.2, §6.1; docs/classic-sets.md B5 E35): can't be
// attacked, attacked only from its own lane, can't attack or be attacked, Immune to Spells' "doesn't
// affect it" half, a keyword that holds only while a condition does, and the forced attacks on a
// random enemy and on the unit's own hero — which skip §4.2 steps 1 to 3 (R53) but obey these.
//
// Port of `packages/engine/test/restrictions.test.ts`.

use jackioh_engine::effects::{
    damage, damage_all, destroy, forced_attack_own_hero, forced_attack_random, forced_attacks_on, plague,
    vanilla,
};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::damage_combat::{
    answer, ask_controller, bolt, charger, fighter, grunt, note, notes, playing, recorder, replays_to,
    round_trip, statue, storm, top_loser, wall, warded,
};
use crate::rules::fixtures::harness::{events_of_type, in_hand, put, slot};

fn unit_of(instance: &CardInstance) -> AttackTarget {
    AttackTarget::Unit {
        instance: instance.clone(),
    }
}

fn hero_of(player: PlayerId) -> AttackTarget {
    AttackTarget::Hero { player }
}

/// A 1/1 whose Death asks its controller something, so a run of forced attacks pauses between two.
fn asker() -> CardDef {
    json_as(json!({
        "id": "dc-asker",
        "index": "4799",
        "name": "asker (damage and combat)",
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "attack": 1, "health": 1, "keywords": [], "text": "asker" },
        "radiant": { "attack": 2, "health": 2, "keywords": [], "text": "asker" },
    }))
}

fn brute() -> CardDef {
    let mut def = asker();
    def.id = "dc-brute".into();
    def.index = "4798".into();
    def.name = "brute".into();
    def.base = json_as(json!({ "attack": 9, "health": 9, "keywords": [], "text": "brute" }));
    def.radiant = json_as(json!({ "attack": 18, "health": 18, "keywords": [], "text": "brute" }));
    def
}

fn asker_scripts() -> CardScripts {
    let mut resume = IndexMap::new();
    resume.insert("answered", hook(|_ctx| vec![note("asker:answered")]));
    CardScripts {
        base: Script {
            death: Some(hook(|_ctx| vec![note("asker:death"), ask_controller("answered")])),
            resume,
            ..Script::default()
        },
        radiant: Script::default(),
    }
}

fn with_asker() {
    let mut catalog = registered_catalog().clone();
    for def in [asker(), brute()] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    registry.insert(asker().id, asker_scripts());
    register_scripts(registry);
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
        let me = me.map(|card| {
            find_instance(&self.state, &card.id)
                .cloned()
                .unwrap_or_else(|| card.clone())
        });
        let mut sink = self.sink();
        let mut ctx = make_context(&mut sink, me.as_ref(), options);
        apply_effects(&effects, &mut ctx);
    }
}

/// The card as it stands in the state now (TS held the live object).
fn live<'a>(state: &'a GameState, card: &CardInstance) -> &'a CardInstance {
    find_instance(state, &card.id).expect("the card is still in the game")
}

fn live_mut<'a>(state: &'a mut GameState, card: &CardInstance) -> &'a mut CardInstance {
    find_instance_mut(state, &card.id).expect("the card is still in the game")
}

fn input(body: Value) -> ActionInput {
    json_as(body)
}

fn refusal(result: Result<(), EngineError>) -> Option<String> {
    result.err().map(|error| error.message)
}

fn target_ids(targets: &[AttackTarget]) -> Vec<String> {
    targets
        .iter()
        .map(|target| match target {
            AttackTarget::Unit { instance, .. } => instance.id.clone(),
            AttackTarget::Hero { player, .. } => player.to_string(),
        })
        .collect()
}

fn of_type(events: &[GameEvent], kind: GameEventType) -> Vec<Value> {
    events_of_type(events, kind)
        .into_iter()
        .map(|event| serde_json::to_value(event).unwrap())
        .collect()
}

fn on_instance(id: &str) -> Value {
    json!({ "of": "instance", "instanceId": id })
}

mod e35_attack_restrictions_section_4_2_step_2 {
    use super::*;

    #[test]
    fn cant_be_attacked_never_a_target_of_a_declared_or_a_forced_attack_still_one_of_effects() {
        let mut state = playing("dc-cant-be-attacked");
        let attacker = put(
            &mut state,
            &grunt.id,
            slot(PlayerId::P1, Row::Units, 1),
            Default::default(),
        );
        let jet = put(
            &mut state,
            &fighter.id,
            slot(PlayerId::P2, Row::Units, 1),
            Default::default(),
        );
        assert_eq!(
            target_ids(&attack_targets(&state, &attacker)),
            vec!["p2".to_string()]
        );
        assert_eq!(
            refusal(why_cannot_attack(&state, &attacker, &unit_of(&jet))),
            Some("that unit cannot be attacked".to_string())
        );
        let refused = reduce(
            &state,
            &json_as(json!({
                "type": "attack", "attackerId": attacker.id, "targetId": jet.id, "playerId": "p1", "nonce": "dc-x"
            })),
        );
        assert_eq!(refused.error.as_deref(), Some("that unit cannot be attacked"));
        // A forced attack on it does not happen, in silence.
        let mut b = Bench::sink_for(state);
        b.apply(
            Some(&attacker),
            HookOptions::default(),
            vec![forced_attacks_on(json_as(
                json!({ "target": on_instance(&jet.id), "attackers": "self" }),
            ))],
        );
        assert!(of_type(&b.events, GameEventType::AttackDeclared).is_empty());
        // Effects still reach it.
        b.apply(
            Some(&attacker),
            HookOptions::default(),
            vec![damage(json_as(
                json!({ "to": on_instance(&jet.id), "amount": 1 }),
            ))],
        );
        assert_eq!(live(&b.state, &jet).damage, 1);
        // A Taunt it gains binds nobody, since nobody may attack it.
        live_mut(&mut b.state, &jet).granted_keywords = vec![Keyword::Taunt];
        assert!(can_attack(
            &b.state,
            live(&b.state, &attacker),
            &hero_of(PlayerId::P2)
        ));
    }

    #[test]
    fn attacked_only_from_its_lane_its_taunt_binds_only_the_attackers_that_may_reach_it() {
        let mut state = playing("dc-lane-only");
        let far = put(
            &mut state,
            &grunt.id,
            slot(PlayerId::P1, Row::Units, 1),
            Default::default(),
        );
        let near = put(
            &mut state,
            &grunt.id,
            slot(PlayerId::P1, Row::Units, 3),
            Default::default(),
        );
        let top = put(
            &mut state,
            &top_loser.id,
            slot(PlayerId::P2, Row::Units, 3),
            Default::default(),
        );
        assert_eq!(
            refusal(why_cannot_attack(&state, &far, &unit_of(&top))),
            Some("only a unit in its lane may attack that unit".to_string())
        );
        // The Taunt it prints binds only the lane-3 attacker, which must attack it.
        assert!(can_attack(&state, &far, &hero_of(PlayerId::P2)));
        assert!(!can_attack(&state, &near, &hero_of(PlayerId::P2)));
        assert!(can_attack(&state, &near, &unit_of(&top)));
        let mut game = recorder(&state);
        game.play(input(
            json!({ "type": "attack", "attackerId": near.id, "targetId": top.id, "playerId": "p1" }),
        ));
        game.play(input(
            json!({ "type": "attack", "attackerId": far.id, "targetId": "hero-p2", "playerId": "p1" }),
        ));
        assert_eq!(game.state().players.p2.hero.health, 28);
        assert!(replays_to(&game.start, &game.log, game.state()));
    }

    #[test]
    fn cant_attack_or_be_attacked_a_restriction_from_where_a_card_stands_is_registered_by_the_module_that_knows_it()
     {
        let mut state = playing("dc-statue");
        let stone = put(
            &mut state,
            &statue.id,
            slot(PlayerId::P1, Row::Units, 1),
            Default::default(),
        );
        let attacker = put(
            &mut state,
            &grunt.id,
            slot(PlayerId::P2, Row::Units, 2),
            Default::default(),
        );
        let other = put(
            &mut state,
            &grunt.id,
            slot(PlayerId::P1, Row::Units, 2),
            Default::default(),
        );
        state.active = PlayerId::P2;
        assert_eq!(
            refusal(why_cannot_attack(&state, &stone, &hero_of(PlayerId::P2))),
            Some("that unit cannot attack".to_string())
        );
        assert_eq!(
            target_ids(&attack_targets(&state, &attacker)),
            vec![other.id.clone(), "p1".to_string()]
        );
        // TS l.114–120 registered a test-only attack bar with `registerAttackBar`, which SURFACE §6.6
        // does not port (it was never registered by the engine): those assertions are in
        // `.fullsend/notes/spec-gaps-part-26-5.md`. With no bar, both targets stay.
        assert_eq!(attack_targets(&state, &attacker).len(), 2);
    }
}

mod e35_immune_to_spells_a_spells_effects_pass_it_by {
    use super::*;

    #[test]
    fn a_spells_single_target_and_its_sweep_skip_it_a_units_reaches_it() {
        let mut state = playing("dc-immune");
        let shielded = put(
            &mut state,
            &warded.id,
            slot(PlayerId::P2, Row::Units, 1),
            Default::default(),
        );
        let open = put(
            &mut state,
            &wall.id,
            slot(PlayerId::P2, Row::Units, 2),
            Default::default(),
        );
        let sweep = in_hand(&mut state, &storm.id, PlayerId::P1, 1).remove(0);
        // The recorder plays on from this state; TS's `state` stayed the state before the play.
        let mut game = recorder(&state.clone());
        game.play(input(
            json!({ "type": "play", "instanceId": sweep.id, "playerId": "p1" }),
        ));
        assert_eq!(
            find_instance(game.state(), &shielded.id).map(|c| c.damage),
            Some(0)
        );
        assert_eq!(find_instance(game.state(), &open.id).map(|c| c.damage), Some(2));
        assert!(replays_to(&game.start, &game.log, game.state()));

        let mut b = Bench::sink_for(state);
        let spell = in_hand(&mut b.state, &bolt.id, PlayerId::P1, 1).remove(0);
        b.apply(
            Some(&spell),
            HookOptions {
                controller: Some(PlayerId::P1),
                targets: Some(vec![Selection::Instance {
                    instance_id: shielded.id.clone(),
                }]),
                ..Default::default()
            },
            vec![
                damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 3 }))),
                destroy(json_as(json!({ "target": { "of": "chosen" } }))),
            ],
        );
        assert_eq!(live(&b.state, &shielded).damage, 0);
        assert!(live(&b.state, &shielded).marked_destroyed.is_none());
        let body = put(
            &mut b.state,
            &grunt.id,
            slot(PlayerId::P1, Row::Units, 1),
            Default::default(),
        );
        b.apply(
            Some(&body),
            HookOptions::default(),
            vec![damage_all(json_as(json!({ "amount": 1, "side": "enemy" })))],
        );
        assert_eq!(live(&b.state, &shielded).damage, 1);
    }

    #[test]
    fn a_continuation_whose_spell_has_gone_is_still_a_spells_its_definition_and_the_radiant_top_loser_is_immune()
     {
        let mut state = playing("dc-immune-ctx");
        let radiant_top = put(
            &mut state,
            &top_loser.id,
            slot(PlayerId::P2, Row::Units, 1),
            json_as(json!({ "radiant": true })),
        );
        let top = live(&state, &radiant_top).clone();
        let mut b = Bench::sink_for(state);
        // TS `{ ...makeContext(sink, null, { controller: "p1" }), defId: bolt.id, radiant: false }`.
        let mut sink = b.sink();
        let mut gone = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(PlayerId::P1),
                ..Default::default()
            },
        );
        gone.def_id = Some(bolt.id.clone());
        gone.radiant = false;
        assert!(effect_is_from_spell(&gone));
        assert!(unaffected_by(&gone, &top));
        // `{ ...gone, defId: grunt.id }`.
        gone.def_id = Some(grunt.id.clone());
        assert!(!unaffected_by(&gone, &top));
        gone.def_id = Some(bolt.id.clone());
        let plain_top = put(
            gone.state,
            &top_loser.id,
            slot(PlayerId::P2, Row::Units, 2),
            Default::default(),
        );
        assert!(!unaffected_by(&gone, &plain_top));
    }
}

mod e35_a_keyword_that_holds_only_while_a_condition_does {
    use super::*;

    fn keywords(state: &GameState, card: &CardInstance) -> Vec<KeywordKind> {
        unit_view(state, live(state, card))
            .keywords
            .iter()
            .map(|keyword| keyword.kind())
            .collect()
    }

    #[test]
    fn first_strike_while_it_has_a_plague_counter_gone_when_the_token_is_and_with_the_text_under_a_vanilla() {
        let mut state = playing("dc-conditional");
        let bull = put(
            &mut state,
            &charger.id,
            slot(PlayerId::P1, Row::Units, 1),
            Default::default(),
        );
        assert!(!keywords(&state, &bull).contains(&KeywordKind::FirstStrike));
        let mut b = Bench::sink_for(state);
        b.apply(
            Some(&bull),
            HookOptions::default(),
            vec![plague(json_as(
                json!({ "target": { "of": "self" }, "amount": 1 }),
            ))],
        );
        assert!(keywords(&b.state, &bull).contains(&KeywordKind::FirstStrike));
        // It strikes first: the 2/2 it attacks dies before hitting back.
        let victim = put(
            &mut b.state,
            &grunt.id,
            slot(PlayerId::P2, Row::Units, 1),
            Default::default(),
        );
        // The recorder plays on from this state; TS's `state` stayed the state before the attack.
        let mut game = recorder(&b.state.clone());
        game.play(input(
            json!({ "type": "attack", "attackerId": bull.id, "targetId": victim.id, "playerId": "p1" }),
        ));
        assert_eq!(find_instance(game.state(), &bull.id).map(|c| c.damage), Some(0));
        assert!(replays_to(&game.start, &game.log, game.state()));
        b.apply(
            Some(&bull),
            HookOptions::default(),
            vec![vanilla(json_as(json!({ "target": { "of": "self" } })))],
        );
        assert!(!keywords(&b.state, &bull).contains(&KeywordKind::FirstStrike));
    }
}

mod e35_forced_attacks_on_a_random_enemy_and_on_the_units_own_hero {
    use super::*;

    #[test]
    fn a_random_enemy_is_drawn_from_the_targets_the_attacker_may_attack_an_enemy_unit_leaves_the_hero_out() {
        let mut state = playing("dc-random");
        let attacker = put(
            &mut state,
            &grunt.id,
            slot(PlayerId::P1, Row::Units, 1),
            Default::default(),
        );
        put(
            &mut state,
            &fighter.id,
            slot(PlayerId::P2, Row::Units, 1),
            Default::default(),
        );
        let top = put(
            &mut state,
            &top_loser.id,
            slot(PlayerId::P2, Row::Units, 3),
            Default::default(),
        );
        let open = put(
            &mut state,
            &wall.id,
            slot(PlayerId::P2, Row::Units, 2),
            Default::default(),
        );
        let ids = |state: &GameState, among: &str| -> Vec<String> {
            target_ids(&random_attack_targets(
                state,
                live(state, &attacker),
                json_as(json!(among)),
            ))
        };
        // Not the unattackable fighter, not the lane-3 Top Loser from lane 1; Taunt and sickness waived.
        assert_eq!(ids(&state, "enemyUnits"), vec![open.id.clone()]);
        assert_eq!(ids(&state, "enemies"), vec![open.id.clone(), "p2".to_string()]);
        let turn = state.turn;
        {
            let card = live_mut(&mut state, &attacker);
            card.summoned_turn = Some(turn);
            card.position = Some(Position::Def);
        }
        let mut b = Bench::sink_for(state);
        b.apply(
            Some(&attacker),
            HookOptions::default(),
            vec![forced_attack_random(json_as(
                json!({ "attacker": { "of": "self" }, "among": "enemyUnits" }),
            ))],
        );
        assert_eq!(
            of_type(&b.events, GameEventType::AttackDeclared),
            vec![
                json!({ "type": "attackDeclared", "attackerId": attacker.id, "targetId": open.id, "forced": true })
            ]
        );
        assert_eq!(live(&b.state, &top).damage, 0);
        // With nothing it may attack, nothing happens.
        let mut lonely = playing("dc-random-none");
        let striker = put(
            &mut lonely,
            &grunt.id,
            slot(PlayerId::P1, Row::Units, 1),
            Default::default(),
        );
        put(
            &mut lonely,
            &fighter.id,
            slot(PlayerId::P2, Row::Units, 1),
            Default::default(),
        );
        let mut quiet = Bench::sink_for(lonely);
        quiet.apply(
            Some(&striker),
            HookOptions::default(),
            vec![forced_attack_random(json_as(
                json!({ "attacker": { "of": "self" }, "among": "enemyUnits", "times": 3 }),
            ))],
        );
        assert!(quiet.events.is_empty());
    }

    #[test]
    fn r113_a_death_that_asks_between_two_random_attacks_owes_the_rest_which_survives_a_round_trip() {
        let mut state = playing("dc-random-pause");
        with_asker();
        let attacker = put(
            &mut state,
            &brute().id,
            slot(PlayerId::P1, Row::Units, 1),
            Default::default(),
        );
        // The first draw picks the asker: it stands where the match rng's first pick lands.
        let first_pick = Rng::new(&state.seed, state.rng_cursor).int(2);
        let asker_card = put(
            &mut state,
            &asker().id,
            slot(PlayerId::P2, Row::Units, if first_pick == 0 { 1 } else { 2 }),
            Default::default(),
        );
        let plain = put(
            &mut state,
            &wall.id,
            slot(PlayerId::P2, Row::Units, if first_pick == 0 { 2 } else { 1 }),
            Default::default(),
        );
        let mut b = Bench::sink_for(state);
        let striker = live(&b.state, &attacker).clone();
        force_attacks_random(&mut b.sink(), &striker, json_as(json!("enemyUnits")), 2, None);
        assert_eq!(notes(&b.state), vec!["asker:death".to_string()]);
        assert_eq!(b.state.pending.as_ref().map(|p| p.player_id), Some(PlayerId::P2));
        let owed = owed_work(&b.state, Some(FORCED_RANDOM_WORK));
        assert_eq!(owed.len(), 1);
        let run = owed[0].resume.data.get("run").cloned().unwrap_or(Value::Null);
        assert_eq!(run["attacker"], json!(attacker.id));
        assert_eq!(run["among"], json!("enemyUnits"));
        assert_eq!(run["left"], json!(1));
        assert!(run["since"].is_number(), "{run}");
        assert_eq!(run.as_object().map(|fields| fields.len()), Some(4), "{run}");
        b.state.rng_cursor = b.rng.cursor();
        let resumed = answer(&round_trip(&b.state)).state;
        assert_eq!(
            notes(&resumed),
            vec!["asker:death".to_string(), "asker:answered".to_string()]
        );
        // The second attack went to the one enemy Unit left.
        assert_eq!(
            find_instance(&resumed, &plain.id).map(|c| c.zone.z()),
            Some(ZoneName::Graveyard)
        );
        assert_eq!(
            find_instance(&resumed, &asker_card.id).map(|c| c.zone.z()),
            Some(ZoneName::Graveyard)
        );
        assert!(resumed.work.is_empty());
    }

    #[test]
    fn r96_the_radiants_second_attack_happens_only_while_the_unit_is_still_on_the_field() {
        let mut state = playing("dc-random-gone");
        let attacker = put(
            &mut state,
            &grunt.id,
            slot(PlayerId::P1, Row::Units, 1),
            Default::default(),
        );
        let spiky = put(
            &mut state,
            &grunt.id,
            slot(PlayerId::P2, Row::Units, 2),
            Default::default(),
        );
        live_mut(&mut state, &spiky).buffs = AttackHealth {
            attack: 5,
            health: 20,
        };
        let mut b = Bench::sink_for(state);
        let striker = live(&b.state, &attacker).clone();
        force_attacks_random(&mut b.sink(), &striker, json_as(json!("enemyUnits")), 2, None);
        // The first strike back killed it, so there is no second attack.
        let targets: Vec<Value> = of_type(&b.events, GameEventType::AttackDeclared)
            .into_iter()
            .map(|event| event["targetId"].clone())
            .collect();
        assert_eq!(targets, vec![json!(spiky.id)]);
        assert_eq!(
            find_instance(&b.state, &attacker.id).map(|c| c.zone.z()),
            Some(ZoneName::Graveyard)
        );
    }

    #[test]
    fn a_forced_attack_on_its_own_hero_the_hero_takes_its_attack_and_strikes_nothing_back() {
        let mut state = playing("dc-own-hero");
        let attacker = put(
            &mut state,
            &grunt.id,
            slot(PlayerId::P1, Row::Units, 1),
            Default::default(),
        );
        live_mut(&mut state, &attacker).buffs = AttackHealth { attack: 3, health: 0 };
        let mut b = Bench::sink_for(state);
        b.apply(
            Some(&attacker),
            HookOptions::default(),
            vec![forced_attack_own_hero(json_as(
                json!({ "attacker": { "of": "self" } }),
            ))],
        );
        assert_eq!(b.state.players.p1.hero.health, 25);
        assert_eq!(live(&b.state, &attacker).damage, 0);
        assert_eq!(
            of_type(&b.events, GameEventType::AttackDeclared),
            vec![
                json!({ "type": "attackDeclared", "attackerId": attacker.id, "targetId": "hero-p1", "forced": true })
            ]
        );
        // A unit that cannot attack does not.
        let stone = put(
            &mut b.state,
            &statue.id,
            slot(PlayerId::P1, Row::Units, 2),
            Default::default(),
        );
        b.apply(
            Some(&stone),
            HookOptions::default(),
            vec![forced_attack_own_hero(json_as(
                json!({ "attacker": { "of": "self" } }),
            ))],
        );
        assert_eq!(b.state.players.p1.hero.health, 25);
    }
}
