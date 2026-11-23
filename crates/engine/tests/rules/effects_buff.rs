//! The buff and grantKeyword effects (BUILD M3-T1): layer 4 of §10.4, under the auras of layer 5,
//! plus §6.1's keyword set, Armor summing across sources, and R21's random keyword pool.
//! The all-keyword fixture unit this file needs is defined and registered here (BUILD §0).

use jackioh_engine::effects::{
    buff, buff_all_units, grant_keyword, grant_random_keywords, random_pool_keywords,
};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{indestructible, plain, shielded, spikey_pillow, taunter, zero_attack};
use crate::rules::fixtures::harness::{Bench, events_of_type, live, live_mut, new_game, put, slot};

/// A unit holding every keyword in R21's pool, so the pool can run dry.
fn pool_keywords() -> Value {
    // R781005: the combat keyword pool as the engine holds it, so the fixture never restates it.
    serde_json::to_value(random_pool_keywords()).expect("keywords serialize")
}

const EVERY_KEYWORD: &str = "bf-every-keyword";

fn every_keyword() -> CardDef {
    json_as(json!({
        "id": EVERY_KEYWORD,
        "index": "901",
        "name": "Every Keyword (buff fixture)",
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "attack": 2, "health": 2, "keywords": pool_keywords(), "text": "every pool keyword" },
        "radiant": { "attack": 4, "health": 4, "keywords": pool_keywords(), "text": "every pool keyword" },
    }))
}

fn game(seed: &str) -> GameState {
    let state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    catalog.insert(EVERY_KEYWORD.to_string(), every_keyword());
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    scripts.insert(EVERY_KEYWORD.to_string(), CardScripts::default());
    register_scripts(scripts);
    state
}

fn view(state: &GameState, id: &str) -> UnitView {
    unit_view(state, &live(state, id))
}

/// Apply one effect the way the engine does (§10.9).
fn run(
    bench: &mut Bench,
    state: &mut GameState,
    effect: Effect,
    self_: Option<CardInstance>,
    options: HookOptions,
) {
    let mut sink = bench.sink(state);
    let mut ctx = make_context(&mut sink, self_.as_ref(), options);
    (effect.apply)(&mut ctx);
}

fn on_instance(instance: &CardInstance) -> HookOptions {
    HookOptions {
        targets: Some(vec![Selection::Instance {
            instance_id: instance.id.clone(),
        }]),
        ..Default::default()
    }
}

fn as_player(player: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(player),
        ..Default::default()
    }
}

fn granted_kinds(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::KeywordGranted { keyword, .. } => Some(keyword.kind().as_str().to_string()),
            _ => None,
        })
        .collect()
}

fn buffed_json(events: &[GameEvent]) -> Value {
    let buffed: Vec<&GameEvent> = events
        .iter()
        .filter(|event| event.event_type() == GameEventType::Buffed)
        .collect();
    serde_json::to_value(buffed).unwrap()
}

fn hit_unit(bench: &mut Bench, state: &mut GameState, id: &str, amount: i32) -> i32 {
    let instance = live(state, id);
    deal_damage(
        &mut bench.sink(state),
        DamageArgs {
            source: None,
            target: DamageTarget::Unit { instance },
            amount,
            flags: None,
        },
    )
}

const NO_BUFF: AttackHealth = AttackHealth { attack: 0, health: 0 };

mod buff_s10_4_layer_4_m3_t1 {
    use super::*;

    #[test]
    fn adds_to_buffs_attack_and_buffs_health_permanently_and_emits_the_change() {
        let mut state = game("buff-basic");
        let unit = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        ); // 3/3
        let mut b = Bench::new(&state);

        run(
            &mut b,
            &mut state,
            buff(json_as(
                json!({ "target": { "of": "chosen" }, "attack": 2, "health": 3 }),
            )),
            None,
            on_instance(&unit),
        );

        assert_eq!(
            live(&state, &unit.id).buffs,
            AttackHealth { attack: 2, health: 3 }
        );
        assert_eq!(view(&state, &unit.id).attack, 5);
        assert_eq!(view(&state, &unit.id).max_health, 6);
        assert_eq!(view(&state, &unit.id).health, 6);
        assert_eq!(
            buffed_json(&b.events),
            json!([{ "type": "buffed", "instanceId": unit.id, "attack": 2, "health": 3 }])
        );

        // A second buff accumulates, and a damaged unit keeps its damage: health = max − damage.
        hit_unit(&mut b, &mut state, &unit.id, 4);
        let me = Some(live(&state, &unit.id));
        run(
            &mut b,
            &mut state,
            buff(json_as(
                json!({ "target": { "of": "self" }, "attack": 1, "health": 1 }),
            )),
            me,
            HookOptions::default(),
        );
        assert_eq!(
            live(&state, &unit.id).buffs,
            AttackHealth { attack: 3, health: 4 }
        );
        assert_eq!(view(&state, &unit.id).max_health, 7);
        assert_eq!(view(&state, &unit.id).health, 3);
        assert_eq!(events_of_type(&b.events, GameEventType::Buffed).len(), 2);
    }

    #[test]
    fn is_layer_4_so_an_aura_adds_on_top_of_it_without_touching_the_stored_buff_s10_4_layer_5() {
        let mut state = game("buff-layers");
        let unit = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        ); // 3/3
        let mut b = Bench::new(&state);

        let me = Some(live(&state, &unit.id));
        run(
            &mut b,
            &mut state,
            buff(json_as(
                json!({ "target": { "of": "self" }, "attack": 3, "health": 3 }),
            )),
            me,
            HookOptions::default(),
        );
        assert_eq!(view(&state, &unit.id).attack, 6);

        // Spikey Pillow's aura: your units have −2 attack, applied after the buff.
        put(
            &mut state,
            &spikey_pillow.id,
            slot(PlayerId::P1, Row::Units, 2),
            json!({}),
        );
        assert_eq!(view(&state, &unit.id).attack, 4);
        assert_eq!(live(&state, &unit.id).buffs.attack, 3);

        // §10.4: attack floors at 0 in the aura layer, and the buff below it is still on the instance.
        let weakling = put(
            &mut state,
            &zero_attack.id,
            slot(PlayerId::P1, Row::Units, 3),
            json!({}),
        ); // 0/8
        let me = Some(live(&state, &weakling.id));
        run(
            &mut b,
            &mut state,
            buff(json_as(json!({ "target": { "of": "self" }, "attack": 1 }))),
            me,
            HookOptions::default(),
        );
        assert_eq!(view(&state, &weakling.id).attack, 0);
        assert_eq!(live(&state, &weakling.id).buffs.attack, 1);
    }

    #[test]
    fn r78_a_buff_is_dropped_when_the_card_leaves_the_field() {
        let mut state = game("buff-leaves");
        let unit = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let mut b = Bench::new(&state);

        let me = Some(live(&state, &unit.id));
        run(
            &mut b,
            &mut state,
            buff(json_as(
                json!({ "target": { "of": "self" }, "attack": 4, "health": 4 }),
            )),
            me,
            HookOptions::default(),
        );
        assert_eq!(
            live(&state, &unit.id).buffs,
            AttackHealth { attack: 4, health: 4 }
        );

        let mut now = live(&state, &unit.id);
        move_to_zone(&mut state, &mut now, OffFieldZone::Hand, Default::default());
        assert_eq!(live(&state, &unit.id).buffs, NO_BUFF);
    }

    #[test]
    fn buffs_every_unit_you_control_in_lane_order_and_leaves_the_enemy_board_alone() {
        let mut state = game("buff-all");
        let first = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let third = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 3),
            json!({}),
        );
        let enemy = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let mut b = Bench::new(&state);

        run(
            &mut b,
            &mut state,
            buff_all_units(json_as(json!({ "attack": 1, "health": 1 }))),
            None,
            as_player(PlayerId::P1),
        );

        let buffed_ids: Vec<String> = b
            .events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Buffed { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(buffed_ids, vec![first.id.clone(), third.id.clone()]);
        assert_eq!(view(&state, &first.id).attack, 4);
        assert_eq!(view(&state, &third.id).attack, 4);
        assert_eq!(live(&state, &enemy.id).buffs, NO_BUFF);

        // The enemy form reaches only the other side.
        run(
            &mut b,
            &mut state,
            buff_all_units(json_as(json!({ "side": "enemy", "attack": 2 }))),
            None,
            as_player(PlayerId::P1),
        );
        assert_eq!(
            live(&state, &enemy.id).buffs,
            AttackHealth { attack: 2, health: 0 }
        );
        assert_eq!(
            live(&state, &first.id).buffs,
            AttackHealth { attack: 1, health: 1 }
        );
    }

    #[test]
    fn does_nothing_to_a_hero_target_and_a_0_0_buff_emits_nothing() {
        let mut state = game("buff-fizzle");
        let unit = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let mut b = Bench::new(&state);

        run(
            &mut b,
            &mut state,
            buff(json_as(
                json!({ "target": { "of": "enemyHero" }, "attack": 5, "health": 5 }),
            )),
            None,
            as_player(PlayerId::P1),
        );
        run(
            &mut b,
            &mut state,
            buff(json_as(json!({ "target": { "of": "chosen" }, "attack": 5 }))),
            None,
            HookOptions {
                targets: Some(vec![]),
                ..Default::default()
            },
        );
        let me = Some(live(&state, &unit.id));
        run(
            &mut b,
            &mut state,
            buff(json_as(
                json!({ "target": { "of": "self" }, "attack": 0, "health": 0 }),
            )),
            me,
            HookOptions::default(),
        );

        assert_eq!(b.events.len(), 0);
        assert_eq!(live(&state, &unit.id).buffs, NO_BUFF);
        assert_eq!(
            state.players[PlayerId::P2].hero,
            HeroState { health: 30, armor: 0 }
        );
    }
}

mod grant_keyword_s6_1_s10_4_m3_t1 {
    use super::*;

    #[test]
    fn adds_to_granted_keywords_shows_in_the_view_and_emits_keyword_granted() {
        let mut state = game("grant-basic");
        let unit = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let mut b = Bench::new(&state);

        run(
            &mut b,
            &mut state,
            grant_keyword(json_as(
                json!({ "target": { "of": "chosen" }, "keyword": { "kind": "Taunt" } }),
            )),
            None,
            on_instance(&unit),
        );

        assert_eq!(live(&state, &unit.id).granted_keywords, vec![Keyword::Taunt]);
        assert!(unit_has(&state, &live(&state, &unit.id), KeywordKind::Taunt));
        let granted: Vec<&GameEvent> = b
            .events
            .iter()
            .filter(|event| event.event_type() == GameEventType::KeywordGranted)
            .collect();
        assert_eq!(
            serde_json::to_value(granted).unwrap(),
            json!([{ "type": "keywordGranted", "instanceId": unit.id, "keyword": { "kind": "Taunt" } }])
        );

        // Keywords are a set (§10.4): the same kind is not stored twice.
        let me = Some(live(&state, &unit.id));
        run(
            &mut b,
            &mut state,
            grant_keyword(json_as(
                json!({ "target": { "of": "self" }, "keyword": { "kind": "Taunt" } }),
            )),
            me,
            HookOptions::default(),
        );
        assert_eq!(live(&state, &unit.id).granted_keywords, vec![Keyword::Taunt]);

        // R78: the grant goes when the card leaves the field.
        let mut now = live(&state, &unit.id);
        move_to_zone(&mut state, &mut now, OffFieldZone::Hand, Default::default());
        assert_eq!(live(&state, &unit.id).granted_keywords, Vec::<Keyword>::new());
    }

    #[test]
    fn armor_sums_across_sources_so_a_granted_armor_1_adds_to_printed_armor_and_defense() {
        let mut state = game("grant-armor");
        let unit = put(
            &mut state,
            &taunter.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let mut b = Bench::new(&state);

        let me = Some(live(&state, &unit.id));
        run(
            &mut b,
            &mut state,
            grant_keyword(json_as(
                json!({ "target": { "of": "self" }, "keyword": { "kind": "Armor", "n": 3 } }),
            )),
            me,
            HookOptions::default(),
        );
        assert_eq!(view(&state, &unit.id).armor, 3);

        // Armor carries a number, so a second grant stacks instead of being folded into the first.
        let me = Some(live(&state, &unit.id));
        run(
            &mut b,
            &mut state,
            grant_keyword(json_as(
                json!({ "target": { "of": "self" }, "keyword": { "kind": "Armor", "n": 1 } }),
            )),
            me,
            HookOptions::default(),
        );
        assert_eq!(view(&state, &unit.id).armor, 4);
        assert_eq!(
            live(&state, &unit.id).granted_keywords,
            vec![Keyword::Armor { n: 3 }, Keyword::Armor { n: 1 }]
        );

        // Defense Position adds its own +1 on top (§4.1).
        live_mut(&mut state, &unit.id).position = Some(Position::Def);
        assert_eq!(view(&state, &unit.id).armor, 5);
        assert_eq!(armor_of(&view(&state, &unit.id).keywords), 5);
    }

    #[test]
    fn granting_divine_shield_to_a_unit_whose_shield_was_spent_makes_the_shield_work_again_s10_4() {
        let mut state = game("grant-shield");
        let unit = put(
            &mut state,
            &shielded.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        ); // 2/2, Divine Shield
        let mut b = Bench::new(&state);

        assert_eq!(hit_unit(&mut b, &mut state, &unit.id, 5), 0);
        assert_eq!(live(&state, &unit.id).divine_shield_spent, Some(true));
        assert!(!unit_has(
            &state,
            &live(&state, &unit.id),
            KeywordKind::DivineShield
        ));

        let me = Some(live(&state, &unit.id));
        run(
            &mut b,
            &mut state,
            grant_keyword(json_as(
                json!({ "target": { "of": "self" }, "keyword": { "kind": "Divine Shield" } }),
            )),
            me,
            HookOptions::default(),
        );

        assert_eq!(live(&state, &unit.id).divine_shield_spent, None);
        assert!(unit_has(
            &state,
            &live(&state, &unit.id),
            KeywordKind::DivineShield
        ));
        assert_eq!(hit_unit(&mut b, &mut state, &unit.id, 5), 0);
        assert_eq!(live(&state, &unit.id).damage, 0);
        assert_eq!(live(&state, &unit.id).divine_shield_spent, Some(true));
    }

    #[test]
    fn granting_reborn_to_a_unit_that_already_used_it_makes_reborn_available_again_s10_4() {
        let mut state = game("grant-reborn");
        let unit = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        find_instance_mut(&mut state, &unit.id)
            .expect("the unit")
            .reborn_spent = Some(true);
        let mut b = Bench::new(&state);

        let me = Some(live(&state, &unit.id));
        run(
            &mut b,
            &mut state,
            grant_keyword(json_as(
                json!({ "target": { "of": "self" }, "keyword": { "kind": "Reborn" } }),
            )),
            me,
            HookOptions::default(),
        );

        assert_eq!(live(&state, &unit.id).reborn_spent, None);
        assert!(unit_has(&state, &live(&state, &unit.id), KeywordKind::Reborn));
    }
}

mod r21_random_keywords_m3_t1 {
    use super::*;

    #[test]
    fn r3551004_a_random_keyword_can_be_frenzy() {
        // R3551004: Frenzy is the pool's fifteenth entry, so a full-pool grant lands it.
        assert!(random_pool_keywords().contains(&Keyword::Frenzy));
        let mut state = game("random-kw-frenzy");
        let unit = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let mut b = Bench::new(&state);
        run(
            &mut b,
            &mut state,
            grant_random_keywords(json_as(json!({ "target": { "of": "chosen" }, "count": 15 }))),
            None,
            on_instance(&unit),
        );
        let kinds: Vec<KeywordKind> = live(&state, &unit.id)
            .granted_keywords
            .iter()
            .map(Keyword::kind)
            .collect();
        assert_eq!(kinds.len(), 15);
        assert!(kinds.contains(&KeywordKind::Frenzy));
    }

    #[test]
    fn r21_draws_from_the_pool_never_repeats_within_one_grant_and_is_seeded() {
        let mut state = game("random-kw");
        let unit = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let mut b = Bench::new(&state);

        run(
            &mut b,
            &mut state,
            grant_random_keywords(json_as(json!({ "target": { "of": "chosen" }, "count": 2 }))),
            None,
            on_instance(&unit),
        );

        let kinds = granted_kinds(&b.events);
        assert_eq!(kinds.len(), 2);
        assert_eq!(kinds.iter().collect::<IndexSet<_>>().len(), 2);
        assert_eq!(live(&state, &unit.id).granted_keywords.len(), 2);
        let pool_kinds: Vec<&str> = random_pool_keywords()
            .iter()
            .map(|keyword| keyword.kind().as_str())
            .collect();
        for kind in &kinds {
            assert!(pool_kinds.contains(&kind.as_str()));
        }
        let shown = view(&state, &unit.id).keywords;
        for keyword in &live(&state, &unit.id).granted_keywords {
            // R347: an Indestructible unit shows no Taunt, whichever came first.
            if keyword.kind() == KeywordKind::Taunt && has_keyword(&shown, KeywordKind::Indestructible) {
                continue;
            }
            assert!(has_keyword(&shown, keyword.kind()));
        }

        // Same seed, same draws: the effect only ever touches ctx.rng (§9.3).
        let mut replay = game("random-kw");
        let same = put(
            &mut replay,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let mut replay_bench = Bench::new(&replay);
        let me = Some(live(&replay, &same.id));
        run(
            &mut replay_bench,
            &mut replay,
            grant_random_keywords(json_as(json!({ "target": { "of": "self" }, "count": 2 }))),
            me,
            HookOptions::default(),
        );
        assert_eq!(
            live(&replay, &same.id).granted_keywords,
            live(&state, &unit.id).granted_keywords
        );
    }

    #[test]
    fn r21_never_grants_a_keyword_the_unit_already_has_from_any_source() {
        let mut state = game("random-kw-held");
        let unit = put(
            &mut state,
            &taunter.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        ); // printed Taunt
        // Defense grants Taunt and Armor 1 (§4.1).
        find_instance_mut(&mut state, &unit.id)
            .expect("the unit")
            .position = Some(Position::Def);
        let mut b = Bench::new(&state);

        // Every draw the 14-entry pool has room for but Taunt, nothing it has (R781005, R1025000, R1615000).
        let me = Some(live(&state, &unit.id));
        let count = RANDOM_KEYWORD_POOL.len() - 1;
        run(
            &mut b,
            &mut state,
            grant_random_keywords(json_as(json!({ "target": { "of": "self" }, "count": count }))),
            me,
            HookOptions::default(),
        );

        let kinds = granted_kinds(&b.events);
        assert!(!kinds.contains(&"Taunt".to_string()));
        assert!(!kinds.contains(&"Armor".to_string()));
        assert_eq!(kinds.iter().collect::<IndexSet<_>>().len(), kinds.len());
        assert_eq!(kinds.len(), RANDOM_KEYWORD_POOL.len() - 1);
    }

    #[test]
    fn r21_r1025000_a_grant_of_the_whole_pool_never_gives_armor_or_spell_damage() {
        let mut state = game("random-kw-no-numbered");
        let unit = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let mut b = Bench::new(&state);

        // Drawn dry on a keywordless unit: every entry, never Armor 1 or Spell Damage +1.
        run(
            &mut b,
            &mut state,
            grant_random_keywords(json_as(
                json!({ "target": { "of": "chosen" }, "count": RANDOM_KEYWORD_POOL.len() }),
            )),
            None,
            on_instance(&unit),
        );

        let kinds = granted_kinds(&b.events);
        assert!(!kinds.contains(&"Armor".to_string()));
        assert!(!kinds.contains(&"Spell Damage".to_string()));
        assert_eq!(kinds.iter().collect::<IndexSet<_>>().len(), kinds.len());
        // Taunt is not held down, since the pool draws no Indestructible (R1615000).
        assert!(kinds.len() >= RANDOM_KEYWORD_POOL.len() - 1);
        for keyword in &live(&state, &unit.id).granted_keywords {
            assert!(!matches!(
                keyword.kind(),
                KeywordKind::Armor | KeywordKind::SpellDamage
            ));
        }
    }

    #[test]
    fn r21_r1615000_a_grant_of_the_whole_pool_never_gives_charge_or_indestructible() {
        let mut state = game("random-kw-no-charge");
        let unit = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let mut b = Bench::new(&state);

        // Drawn dry on a keywordless unit: all fifteen (R3551004: Frenzy the fifteenth), Taunt among them,
        // never Charge or Indestructible.
        run(
            &mut b,
            &mut state,
            grant_random_keywords(json_as(
                json!({ "target": { "of": "chosen" }, "count": RANDOM_KEYWORD_POOL.len() }),
            )),
            None,
            on_instance(&unit),
        );

        let kinds = granted_kinds(&b.events);
        assert_eq!(kinds.len(), 15);
        assert_eq!(kinds.len(), RANDOM_KEYWORD_POOL.len());
        assert_eq!(kinds.iter().collect::<IndexSet<_>>().len(), kinds.len());
        assert!(kinds.contains(&"Taunt".to_string()));
        assert!(!kinds.contains(&"Charge".to_string()));
        assert!(!kinds.contains(&"Indestructible".to_string()));
    }

    #[test]
    fn r781004_r1615000_an_indestructible_unit_is_offered_every_pool_keyword_but_taunt() {
        let mut state = game("random-kw-indestructible");
        let unit = put(
            &mut state,
            &indestructible.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let mut b = Bench::new(&state);

        // Its printed Indestructible is no pool entry, and still keeps Taunt out (R347).
        run(
            &mut b,
            &mut state,
            grant_random_keywords(json_as(
                json!({ "target": { "of": "chosen" }, "count": RANDOM_KEYWORD_POOL.len() }),
            )),
            None,
            on_instance(&unit),
        );

        let kinds = granted_kinds(&b.events);
        assert_eq!(kinds.len(), RANDOM_KEYWORD_POOL.len() - 1);
        assert!(!kinds.contains(&"Taunt".to_string()));
        assert!(!kinds.contains(&"Indestructible".to_string()));
    }

    #[test]
    fn r21_a_unit_holding_the_whole_pool_gets_nothing_and_emits_nothing() {
        let mut state = game("random-kw-full");
        let unit = put(
            &mut state,
            EVERY_KEYWORD,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let mut b = Bench::new(&state);

        let me = Some(live(&state, &unit.id));
        run(
            &mut b,
            &mut state,
            grant_random_keywords(json_as(json!({ "target": { "of": "self" }, "count": 3 }))),
            me,
            HookOptions::default(),
        );

        assert_eq!(live(&state, &unit.id).granted_keywords, Vec::<Keyword>::new());
        assert_eq!(b.events.len(), 0);
    }
}

mod r3778000_combat_keywords_follow_their_number {
    use super::*;
    use jackioh_engine::effects::{
        DrawCombatKeywordsArgs, aura_combat_keywords, clear_radiant, drawn_combat_keywords, flicker_card,
        set_number, set_radiant, top_up_combat_keywords, upgrade,
    };

    use crate::rules::fixtures::harness::{apply_now, both};

    /// R3778000: "Aura: Has {keywords} random combat keywords", 3 on the base face and 6 on the
    /// Radiant one, following its number with no arrival draw, so a test draws its first list by hand.
    const AURA: &str = "bf-aura-keywords";

    fn aura() -> CardDef {
        let text = "Aura: Has {keywords|random combat keyword|random combat keywords}.";
        json_as(json!({
            "id": AURA,
            "index": "902",
            "name": "Aura Keywords (buff fixture)",
            "set": "Core",
            "type": "Unit",
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 1,
            "base": { "attack": 2, "health": 2, "keywords": [], "text": text },
            "radiant": { "attack": 4, "health": 4, "keywords": [], "text": text },
            "params": [{ "key": "keywords", "base": 3, "radiant": 6, "better": "up", "step": 1, "min": 1 }],
        }))
    }

    fn aura_game(seed: &str) -> GameState {
        let state = game(seed);
        let mut catalog = registered_catalog().clone();
        catalog.insert(AURA.to_string(), aura());
        register_catalog(catalog);
        let mut scripts = registered_scripts().clone();
        scripts.insert(
            AURA.to_string(),
            both(Script {
                combat_keywords_follow: Some("keywords"),
                conditional_keywords: Some(read_hook(|args| aura_combat_keywords(&args, "keywords", None))),
                ..Script::default()
            }),
        );
        register_scripts(scripts);
        state
    }

    fn placed(seed: &str) -> (GameState, String) {
        let mut state = aura_game(seed);
        let id = put(&mut state, AURA, slot(PlayerId::P1, Row::Units, 1), json!({})).id;
        (state, id)
    }

    fn number(state: &GameState, id: &str) -> i32 {
        param_value(
            state,
            Some(&live(state, id)),
            "keywords",
            ParamValueOptions::default(),
        )
    }

    /// The arrival's top-up (`top_up_combat_keywords`), for the card's number as it stands.
    fn top_up(b: &mut Bench, state: &mut GameState, id: &str) {
        let me = live(state, id);
        let count = number(state, id);
        run(
            b,
            state,
            top_up_combat_keywords(DrawCombatKeywordsArgs { count, key: None }),
            Some(me),
            HookOptions::default(),
        );
    }

    /// The keywords the card acts with.
    fn shown(state: &GameState, id: &str) -> Vec<KeywordKind> {
        keywords_of(state, &live(state, id))
            .iter()
            .map(Keyword::kind)
            .collect()
    }

    /// The card's list, in draw order.
    fn kept(state: &GameState, id: &str) -> Vec<KeywordKind> {
        drawn_combat_keywords(&live(state, id), None)
            .iter()
            .map(Keyword::kind)
            .collect()
    }

    fn on(id: &str) -> Value {
        json!({ "instanceId": id })
    }

    #[test]
    fn r3778000_a_higher_number_keeps_every_keyword_drawn_and_draws_only_the_rest() {
        let (mut state, id) = placed("aura-up");
        let mut b = Bench::new(&state);
        top_up(&mut b, &mut state, &id);
        let first = kept(&state, &id);
        assert_eq!(first.len(), 3, "{first:?}");
        assert_eq!(shown(&state, &id), first);

        step_param(live_mut(&mut state, &id), "keywords", 1);
        top_up(&mut b, &mut state, &id);

        let now = kept(&state, &id);
        assert_eq!(now.len(), 4, "{now:?}");
        assert_eq!(now[..3], first[..], "the 3 it had stay, in their order");
        assert!(!first.contains(&now[3]), "a 4th, none it had: {now:?}");
        assert_eq!(shown(&state, &id), now, "it acts with all 4");
    }

    #[test]
    fn r3778000_a_lower_number_takes_off_the_newest_and_back_up_returns_it() {
        let (mut state, id) = placed("aura-down");
        let mut b = Bench::new(&state);
        top_up(&mut b, &mut state, &id);
        let list = kept(&state, &id);

        step_param(live_mut(&mut state, &id), "keywords", -1);
        assert_eq!(number(&state, &id), 2);
        let lowered = shown(&state, &id);
        assert_eq!(lowered, list[..2], "the newest, {:?}, comes off", list[2]);
        assert!(!lowered.contains(&list[2]));
        assert_eq!(kept(&state, &id), list, "the list stays whole");

        step_param(live_mut(&mut state, &id), "keywords", 1);
        top_up(&mut b, &mut state, &id);
        assert_eq!(shown(&state, &id), list, "the same one comes back");
        assert_eq!(kept(&state, &id), list, "and nothing new is drawn");
    }

    #[test]
    fn r3778000_a_radiant_face_tops_up_as_it_resolves_and_back_gives_the_same() {
        let (mut state, id) = placed("aura-radiant");
        let mut b = Bench::new(&state);
        top_up(&mut b, &mut state, &id);
        let first = kept(&state, &id);

        apply_now(&mut state, set_radiant(json_as(on(&id))));
        let radiant = kept(&state, &id);
        assert_eq!(radiant.len(), 6, "{radiant:?}");
        assert_eq!(radiant[..3], first[..], "it keeps its 3");
        assert_eq!(shown(&state, &id), radiant);

        apply_now(&mut state, clear_radiant(json_as(on(&id))));
        assert_eq!(shown(&state, &id), first, "the base face's 3 are the first 3");
        assert_eq!(kept(&state, &id), radiant);

        apply_now(&mut state, set_radiant(json_as(on(&id))));
        assert_eq!(kept(&state, &id), radiant, "Radiant again, the same 6");
        assert_eq!(shown(&state, &id), radiant);
    }

    #[test]
    fn r3778000_a_number_set_higher_tops_up_as_it_resolves() {
        let (mut state, id) = placed("aura-set");
        let mut b = Bench::new(&state);
        top_up(&mut b, &mut state, &id);
        let first = kept(&state, &id);

        apply_now(
            &mut state,
            set_number(json_as(
                json!({ "instanceId": id, "which": "param:keywords", "value": 5 }),
            )),
        );

        let now = kept(&state, &id);
        assert_eq!(number(&state, &id), 5);
        assert_eq!(now.len(), 5, "{now:?}");
        assert_eq!(now[..3], first[..]);
        assert_eq!(shown(&state, &id), now);
    }

    #[test]
    fn r3778000_a_buffs_number_row_tops_up_as_it_resolves() {
        let buffed = |seed: &str| {
            let (mut state, id) = placed(seed);
            let mut b = Bench::new(&state);
            top_up(&mut b, &mut state, &id);
            let first = kept(&state, &id);
            apply_now(&mut state, upgrade(json_as(on(&id))));
            (state, id, first)
        };
        // The Buff draws one row of its menu (B3.4 rule 3): the seed is one whose row is the Number's.
        let seed = seed_where("bf-aura-buff", |seed| {
            let (state, id, _) = buffed(seed);
            number(&state, &id) == 4
        });
        let (state, id, first) = buffed(&seed);

        let now = kept(&state, &id);
        assert_eq!(now.len(), 4, "{now:?}");
        assert_eq!(now[..3], first[..]);
        assert_eq!(shown(&state, &id), now);
    }

    #[test]
    fn r3778000_a_rise_queues_nothing_and_a_card_that_does_not_follow_draws_nothing() {
        let (mut state, id) = placed("aura-quiet");
        let mut b = Bench::new(&state);
        top_up(&mut b, &mut state, &id);
        let other = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 2),
            json!({}),
        )
        .id;
        let (seq, cursor) = (state.next_seq, state.rng_cursor);

        apply_now(&mut state, set_radiant(json_as(on(&other))));
        assert_eq!(state.rng_cursor, cursor, "a card with no such Aura draws nothing");
        // R68: the event's dispatch numbers it, and that is all a change takes.
        let per_change = state.next_seq - seq;

        apply_now(&mut state, set_radiant(json_as(on(&id))));
        assert_eq!(kept(&state, &id).len(), 6);
        assert_ne!(state.rng_cursor, cursor, "the Aura drew its rest");
        assert_eq!(
            state.next_seq - seq,
            2 * per_change,
            "and nothing was queued for it"
        );
        assert!(state.trigger_queue.is_empty());
    }

    #[test]
    fn r3778000_a_card_whose_arrival_draw_is_still_to_come_draws_nothing_on_a_rise() {
        let (mut state, id) = placed("aura-unarrived");
        let cursor = state.rng_cursor;

        apply_now(&mut state, set_radiant(json_as(on(&id))));
        assert!(
            kept(&state, &id).is_empty(),
            "no list yet: the arrival draw is still owed"
        );
        assert_eq!(state.rng_cursor, cursor);

        let mut b = Bench::new(&state);
        top_up(&mut b, &mut state, &id);
        assert_eq!(kept(&state, &id).len(), 6, "which draws the number as it is then");
    }

    #[test]
    fn r3778000_with_fewer_left_it_takes_every_one_there_is() {
        let (mut state, id) = placed("aura-short");
        let pool = random_pool_keywords();
        live_mut(&mut state, &id).granted_keywords = pool[2..].to_vec();
        let mut b = Bench::new(&state);

        top_up(&mut b, &mut state, &id);
        let mut list = kept(&state, &id);
        list.sort();
        let mut left = vec![pool[0].kind(), pool[1].kind()];
        left.sort();
        assert_eq!(list, left, "the two left, of the 3 it asks");

        let cursor = b.rng.cursor();
        top_up(&mut b, &mut state, &id);
        assert_eq!(kept(&state, &id).len(), 2, "none is left to draw");
        assert_eq!(b.rng.cursor(), cursor, "and it draws nothing");
    }

    #[test]
    fn r3778000_r347_an_indestructible_unit_never_tops_up_into_taunt() {
        for n in 0..24 {
            let (mut state, id) = placed(&format!("aura-r347-{n}"));
            live_mut(&mut state, &id).granted_keywords = vec![Keyword::Indestructible];
            let mut b = Bench::new(&state);
            top_up(&mut b, &mut state, &id);
            step_param(live_mut(&mut state, &id), "keywords", 3);
            top_up(&mut b, &mut state, &id);
            let list = kept(&state, &id);
            assert_eq!(list.len(), 6, "{list:?}");
            assert!(!list.contains(&KeywordKind::Taunt), "{list:?}");
        }
    }

    #[test]
    fn r3778000_the_auras_copy_still_counts_beside_another_source() {
        let (mut state, id) = placed("aura-both");
        let mut b = Bench::new(&state);
        top_up(&mut b, &mut state, &id);
        let first = kept(&state, &id);
        let granted = drawn_combat_keywords(&live(&state, &id), None)[0].clone();
        live_mut(&mut state, &id).granted_keywords = vec![granted];

        step_param(live_mut(&mut state, &id), "keywords", 1);
        top_up(&mut b, &mut state, &id);

        let now = kept(&state, &id);
        assert_eq!(now.len(), 4, "one more, not two: {now:?}");
        assert_eq!(now[..3], first[..], "the one held twice stays on the list");
        assert!(!first.contains(&now[3]));
        for kind in &now {
            assert!(shown(&state, &id).contains(kind), "{kind:?}");
        }
    }

    #[test]
    fn r3778000_r78_leaving_clears_the_list_and_the_next_draw_is_fresh() {
        let (mut state, id) = placed("aura-leave");
        let mut b = Bench::new(&state);
        step_param(live_mut(&mut state, &id), "keywords", 1);
        top_up(&mut b, &mut state, &id);
        assert_eq!(kept(&state, &id).len(), 4);

        let card = live(&state, &id);
        assert!(b.run(&mut state, |sink| flicker_card(sink, &card)));

        assert!(kept(&state, &id).is_empty(), "R78 clears it");
        assert!(shown(&state, &id).is_empty());
        assert_eq!(
            number(&state, &id),
            4,
            "a Buff is the card's own and stays (R386)"
        );
        top_up(&mut b, &mut state, &id);
        assert_eq!(kept(&state, &id).len(), 4, "a fresh draw of its number");
    }
}
