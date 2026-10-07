//! The buff and grantKeyword effects (BUILD M3-T1): layer 4 of §10.4, under the auras of layer 5,
//! plus §6.1's keyword set, Armor summing across sources, and R21's random keyword pool.
//! The all-keyword fixture unit this file needs is defined and registered here (BUILD §0).
//!
//! Port of `packages/engine/test/effects-buff.test.ts`.

use jackioh_engine::effects::{buff, buff_all_units, grant_keyword, grant_random_keywords};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{plain, shielded, spikey_pillow, taunter, zero_attack};
use crate::rules::fixtures::harness::{events_of_type, new_game, put, slot};

/// A unit holding every keyword in R21's pool, so the pool can run dry.
fn pool_keywords() -> Value {
    json!([
        { "kind": "Taunt" },
        { "kind": "Armor", "n": 1 },
        { "kind": "Rush" },
        { "kind": "Charge" },
        { "kind": "First Strike" },
        { "kind": "Poisonous" },
        { "kind": "Lifesteal" },
        { "kind": "Reborn" },
        { "kind": "Divine Shield" },
        { "kind": "Trample" },
        { "kind": "Cleave" },
        { "kind": "Pierce" },
        { "kind": "Windfury" },
        { "kind": "Deft" },
    ])
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

/// TS `sinkFor(state)`, kept for the whole test: the state, the events every run appends to, and an
/// rng that starts at the state's cursor, as reduce does.
struct Bench {
    state: GameState,
    events: Vec<GameEvent>,
    rng: Rng,
}

impl Bench {
    fn new(state: GameState) -> Bench {
        let rng = Rng::new(&state.seed, state.rng_cursor);
        Bench { state, events: Vec::new(), rng }
    }

    fn sink(&mut self) -> EngineSink<'_> {
        EngineSink::new(&mut self.state, &mut self.events, &mut self.rng)
    }

    /// The card as it stands now (TS held the live object).
    fn card(&self, id: &str) -> CardInstance {
        find_instance(&self.state, id).expect("the card is in the state").clone()
    }

    fn card_mut(&mut self, id: &str) -> &mut CardInstance {
        find_instance_mut(&mut self.state, id).expect("the card is in the state")
    }

    fn view(&self, id: &str) -> UnitView {
        unit_view(&self.state, &self.card(id))
    }
}

/// Apply one effect the way the engine does: a context over the sink, then `apply` (§10.9).
fn run(bench: &mut Bench, effect: Effect, self_: Option<CardInstance>, options: HookOptions) {
    let mut ctx = make_context(bench.sink(), self_, options);
    (effect.apply)(&mut ctx);
}

fn on_instance(instance: &CardInstance) -> HookOptions {
    HookOptions { targets: Some(vec![Selection::Instance { instance_id: instance.id.clone() }]), ..Default::default() }
}

fn as_player(player: PlayerId) -> HookOptions {
    HookOptions { controller: Some(player), ..Default::default() }
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
    let buffed: Vec<&GameEvent> = events.iter().filter(|event| event.event_type() == GameEventType::Buffed).collect();
    serde_json::to_value(buffed).unwrap()
}

fn hit_unit(bench: &mut Bench, id: &str, amount: i32) -> i32 {
    let instance = bench.card(id);
    deal_damage(
        &mut bench.sink(),
        DamageArgs { source: None, target: DamageTarget::Unit { instance }, amount, flags: None },
    )
}

const NO_BUFF: AttackHealth = AttackHealth { attack: 0, health: 0 };

mod buff_s10_4_layer_4_m3_t1 {
    use super::*;

    #[test]
    fn adds_to_buffs_attack_and_buffs_health_permanently_and_emits_the_change() {
        let mut state = game("buff-basic");
        let unit = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({})); // 3/3
        let mut b = Bench::new(state);

        run(
            &mut b,
            buff(json_as(json!({ "target": { "of": "chosen" }, "attack": 2, "health": 3 }))),
            None,
            on_instance(&unit),
        );

        assert_eq!(b.card(&unit.id).buffs, AttackHealth { attack: 2, health: 3 });
        assert_eq!(b.view(&unit.id).attack, 5);
        assert_eq!(b.view(&unit.id).max_health, 6);
        assert_eq!(b.view(&unit.id).health, 6);
        assert_eq!(
            buffed_json(&b.events),
            json!([{ "type": "buffed", "instanceId": unit.id, "attack": 2, "health": 3 }])
        );

        // A second buff accumulates, and a damaged unit keeps its damage: health = max − damage.
        hit_unit(&mut b, &unit.id, 4);
        let me = Some(b.card(&unit.id));
        run(&mut b, buff(json_as(json!({ "target": { "of": "self" }, "attack": 1, "health": 1 }))), me, HookOptions::default());
        assert_eq!(b.card(&unit.id).buffs, AttackHealth { attack: 3, health: 4 });
        assert_eq!(b.view(&unit.id).max_health, 7);
        assert_eq!(b.view(&unit.id).health, 3);
        assert_eq!(events_of_type(&b.events, GameEventType::Buffed).len(), 2);
    }

    #[test]
    fn is_layer_4_so_an_aura_adds_on_top_of_it_without_touching_the_stored_buff_s10_4_layer_5() {
        let mut state = game("buff-layers");
        let unit = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({})); // 3/3
        let mut b = Bench::new(state);

        let me = Some(b.card(&unit.id));
        run(&mut b, buff(json_as(json!({ "target": { "of": "self" }, "attack": 3, "health": 3 }))), me, HookOptions::default());
        assert_eq!(b.view(&unit.id).attack, 6);

        // Spikey Pillow's aura: your units have −2 attack, applied after the buff.
        put(&mut b.state, &spikey_pillow.id, slot(PlayerId::P1, Row::Units, 2), json!({}));
        assert_eq!(b.view(&unit.id).attack, 4);
        assert_eq!(b.card(&unit.id).buffs.attack, 3);

        // §10.4: attack floors at 0 in the aura layer, and the buff below it is still on the instance.
        let weakling = put(&mut b.state, &zero_attack.id, slot(PlayerId::P1, Row::Units, 3), json!({})); // 0/8
        let me = Some(b.card(&weakling.id));
        run(&mut b, buff(json_as(json!({ "target": { "of": "self" }, "attack": 1 }))), me, HookOptions::default());
        assert_eq!(b.view(&weakling.id).attack, 0);
        assert_eq!(b.card(&weakling.id).buffs.attack, 1);
    }

    #[test]
    fn r78_a_buff_is_dropped_when_the_card_leaves_the_field() {
        let mut state = game("buff-leaves");
        let unit = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let mut b = Bench::new(state);

        let me = Some(b.card(&unit.id));
        run(&mut b, buff(json_as(json!({ "target": { "of": "self" }, "attack": 4, "health": 4 }))), me, HookOptions::default());
        assert_eq!(b.card(&unit.id).buffs, AttackHealth { attack: 4, health: 4 });

        let now = b.card(&unit.id);
        move_to_zone(&mut b.state, &now, ZoneName::Hand, Default::default());
        assert_eq!(b.card(&unit.id).buffs, NO_BUFF);
    }

    #[test]
    fn buffs_every_unit_you_control_in_lane_order_and_leaves_the_enemy_board_alone() {
        let mut state = game("buff-all");
        let first = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let third = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 3), json!({}));
        let enemy = put(&mut state, &plain.id, slot(PlayerId::P2, Row::Units, 1), json!({}));
        let mut b = Bench::new(state);

        run(&mut b, buff_all_units(json_as(json!({ "attack": 1, "health": 1 }))), None, as_player(PlayerId::P1));

        let buffed_ids: Vec<String> = b
            .events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Buffed { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(buffed_ids, vec![first.id.clone(), third.id.clone()]);
        assert_eq!(b.view(&first.id).attack, 4);
        assert_eq!(b.view(&third.id).attack, 4);
        assert_eq!(b.card(&enemy.id).buffs, NO_BUFF);

        // The enemy form reaches only the other side.
        run(&mut b, buff_all_units(json_as(json!({ "side": "enemy", "attack": 2 }))), None, as_player(PlayerId::P1));
        assert_eq!(b.card(&enemy.id).buffs, AttackHealth { attack: 2, health: 0 });
        assert_eq!(b.card(&first.id).buffs, AttackHealth { attack: 1, health: 1 });
    }

    #[test]
    fn does_nothing_to_a_hero_target_and_a_0_0_buff_emits_nothing() {
        let mut state = game("buff-fizzle");
        let unit = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let mut b = Bench::new(state);

        run(
            &mut b,
            buff(json_as(json!({ "target": { "of": "enemyHero" }, "attack": 5, "health": 5 }))),
            None,
            as_player(PlayerId::P1),
        );
        run(
            &mut b,
            buff(json_as(json!({ "target": { "of": "chosen" }, "attack": 5 }))),
            None,
            HookOptions { targets: Some(vec![]), ..Default::default() },
        );
        let me = Some(b.card(&unit.id));
        run(&mut b, buff(json_as(json!({ "target": { "of": "self" }, "attack": 0, "health": 0 }))), me, HookOptions::default());

        assert_eq!(b.events.len(), 0);
        assert_eq!(b.card(&unit.id).buffs, NO_BUFF);
        assert_eq!(b.state.players[PlayerId::P2].hero, HeroState { health: 30, armor: 0 });
    }
}

mod grant_keyword_s6_1_s10_4_m3_t1 {
    use super::*;

    #[test]
    fn adds_to_granted_keywords_shows_in_the_view_and_emits_keyword_granted() {
        let mut state = game("grant-basic");
        let unit = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let mut b = Bench::new(state);

        run(
            &mut b,
            grant_keyword(json_as(json!({ "target": { "of": "chosen" }, "keyword": { "kind": "Taunt" } }))),
            None,
            on_instance(&unit),
        );

        assert_eq!(b.card(&unit.id).granted_keywords, vec![Keyword::Taunt]);
        assert!(unit_has(&b.state, &b.card(&unit.id), KeywordKind::Taunt));
        let granted: Vec<&GameEvent> =
            b.events.iter().filter(|event| event.event_type() == GameEventType::KeywordGranted).collect();
        assert_eq!(
            serde_json::to_value(granted).unwrap(),
            json!([{ "type": "keywordGranted", "instanceId": unit.id, "keyword": { "kind": "Taunt" } }])
        );

        // Keywords are a set (§10.4): the same kind is not stored twice.
        let me = Some(b.card(&unit.id));
        run(
            &mut b,
            grant_keyword(json_as(json!({ "target": { "of": "self" }, "keyword": { "kind": "Taunt" } }))),
            me,
            HookOptions::default(),
        );
        assert_eq!(b.card(&unit.id).granted_keywords, vec![Keyword::Taunt]);

        // R78: the grant goes when the card leaves the field.
        let now = b.card(&unit.id);
        move_to_zone(&mut b.state, &now, ZoneName::Hand, Default::default());
        assert_eq!(b.card(&unit.id).granted_keywords, Vec::<Keyword>::new());
    }

    #[test]
    fn armor_sums_across_sources_so_a_granted_armor_1_adds_to_printed_armor_and_defense() {
        // §6.1.
        let mut state = game("grant-armor");
        let unit = put(&mut state, &taunter.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let mut b = Bench::new(state);

        let me = Some(b.card(&unit.id));
        run(
            &mut b,
            grant_keyword(json_as(json!({ "target": { "of": "self" }, "keyword": { "kind": "Armor", "n": 3 } }))),
            me,
            HookOptions::default(),
        );
        assert_eq!(b.view(&unit.id).armor, 3);

        // Armor carries a number, so a second grant stacks instead of being folded into the first.
        let me = Some(b.card(&unit.id));
        run(
            &mut b,
            grant_keyword(json_as(json!({ "target": { "of": "self" }, "keyword": { "kind": "Armor", "n": 1 } }))),
            me,
            HookOptions::default(),
        );
        assert_eq!(b.view(&unit.id).armor, 4);
        assert_eq!(b.card(&unit.id).granted_keywords, vec![Keyword::Armor { n: 3 }, Keyword::Armor { n: 1 }]);

        // Defense Position adds its own +1 on top (§4.1).
        b.card_mut(&unit.id).position = Some(Position::Def);
        assert_eq!(b.view(&unit.id).armor, 5);
        assert_eq!(armor_of(&b.view(&unit.id).keywords), 5);
    }

    #[test]
    fn granting_divine_shield_to_a_unit_whose_shield_was_spent_makes_the_shield_work_again_s10_4() {
        let mut state = game("grant-shield");
        let unit = put(&mut state, &shielded.id, slot(PlayerId::P2, Row::Units, 1), json!({})); // 2/2, Divine Shield
        let mut b = Bench::new(state);

        assert_eq!(hit_unit(&mut b, &unit.id, 5), 0);
        assert_eq!(b.card(&unit.id).divine_shield_spent, Some(true));
        assert!(!unit_has(&b.state, &b.card(&unit.id), KeywordKind::DivineShield));

        let me = Some(b.card(&unit.id));
        run(
            &mut b,
            grant_keyword(json_as(json!({ "target": { "of": "self" }, "keyword": { "kind": "Divine Shield" } }))),
            me,
            HookOptions::default(),
        );

        assert_eq!(b.card(&unit.id).divine_shield_spent, None);
        assert!(unit_has(&b.state, &b.card(&unit.id), KeywordKind::DivineShield));
        assert_eq!(hit_unit(&mut b, &unit.id, 5), 0);
        assert_eq!(b.card(&unit.id).damage, 0);
        assert_eq!(b.card(&unit.id).divine_shield_spent, Some(true));
    }

    #[test]
    fn granting_reborn_to_a_unit_that_already_used_it_makes_reborn_available_again_s10_4() {
        let mut state = game("grant-reborn");
        let unit = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        find_instance_mut(&mut state, &unit.id).expect("the unit").reborn_spent = Some(true);
        let mut b = Bench::new(state);

        let me = Some(b.card(&unit.id));
        run(
            &mut b,
            grant_keyword(json_as(json!({ "target": { "of": "self" }, "keyword": { "kind": "Reborn" } }))),
            me,
            HookOptions::default(),
        );

        assert_eq!(b.card(&unit.id).reborn_spent, None);
        assert!(unit_has(&b.state, &b.card(&unit.id), KeywordKind::Reborn));
    }
}

mod r21_random_keywords_m3_t1 {
    use super::*;

    #[test]
    fn r21_draws_from_the_pool_never_repeats_within_one_grant_and_is_seeded() {
        let mut state = game("random-kw");
        let unit = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let mut b = Bench::new(state);

        run(
            &mut b,
            grant_random_keywords(json_as(json!({ "target": { "of": "chosen" }, "count": 2 }))),
            None,
            on_instance(&unit),
        );

        let kinds = granted_kinds(&b.events);
        assert_eq!(kinds.len(), 2);
        assert_eq!(kinds.iter().collect::<IndexSet<_>>().len(), 2);
        assert_eq!(b.card(&unit.id).granted_keywords.len(), 2);
        let pool_kinds: Vec<&str> =
            RANDOM_KEYWORD_POOL.iter().map(|entry| if *entry == "Armor 1" { "Armor" } else { *entry }).collect();
        for kind in &kinds {
            assert!(pool_kinds.contains(&kind.as_str()));
        }
        for keyword in &b.card(&unit.id).granted_keywords {
            assert!(has_keyword(&b.view(&unit.id).keywords, keyword.kind()));
        }

        // Same seed, same draws: the effect only ever touches ctx.rng (§9.3).
        let mut replay = game("random-kw");
        let same = put(&mut replay, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let mut replay_bench = Bench::new(replay);
        let me = Some(replay_bench.card(&same.id));
        run(
            &mut replay_bench,
            grant_random_keywords(json_as(json!({ "target": { "of": "self" }, "count": 2 }))),
            me,
            HookOptions::default(),
        );
        assert_eq!(replay_bench.card(&same.id).granted_keywords, b.card(&unit.id).granted_keywords);
    }

    #[test]
    fn r21_never_grants_a_keyword_the_unit_already_has_from_any_source() {
        let mut state = game("random-kw-held");
        let unit = put(&mut state, &taunter.id, slot(PlayerId::P1, Row::Units, 1), json!({})); // printed Taunt
        // Defense grants Taunt and Armor 1 (§4.1).
        find_instance_mut(&mut state, &unit.id).expect("the unit").position = Some(Position::Def);
        let mut b = Bench::new(state);

        // 12 draws: everything the 14-entry pool holds but Taunt and Armor, and nothing it has.
        let me = Some(b.card(&unit.id));
        run(
            &mut b,
            grant_random_keywords(json_as(json!({ "target": { "of": "self" }, "count": 12 }))),
            me,
            HookOptions::default(),
        );

        let kinds = granted_kinds(&b.events);
        assert!(!kinds.contains(&"Taunt".to_string()));
        assert!(!kinds.contains(&"Armor".to_string()));
        assert_eq!(kinds.iter().collect::<IndexSet<_>>().len(), kinds.len());
        assert_eq!(kinds.len(), RANDOM_KEYWORD_POOL.len() - 2);
    }

    #[test]
    fn r21_a_unit_holding_the_whole_pool_gets_nothing_and_emits_nothing() {
        let mut state = game("random-kw-full");
        let unit = put(&mut state, EVERY_KEYWORD, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let mut b = Bench::new(state);

        let me = Some(b.card(&unit.id));
        run(
            &mut b,
            grant_random_keywords(json_as(json!({ "target": { "of": "self" }, "count": 3 }))),
            me,
            HookOptions::default(),
        );

        assert_eq!(b.card(&unit.id).granted_keywords, Vec::<Keyword>::new());
        assert_eq!(b.events.len(), 0);
    }
}
