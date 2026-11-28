//! E1 #5236's other keywords (docs/felinors-vs-humans-set.md FV2 "Fragile", "Unlucky X", "Endless
//! Evolve", FV3 E1; R5236010 to R5236031): Fragile's hits, ticks and savers, what a Nerf and a Buff do
//! with it and that R21's pool never draws it; net luck, the worst-of roll and the coin; a Nerf's and a
//! Buff's Unlucky; Endless Evolve; and the keywords the instance view shows.
//!
//! Proved on this file's own fixtures (`fvh-kw-` ids, indexed from 5236200) through the testkit's
//! `scenario()`, on `fixtures::evolution`'s Evolve cards, and on the match rng itself. The new keywords
//! are printed on a fixture's faces as `Keyword` values, never given.

use jackioh_engine::effects::{
    TuneDirection, damage, evolve, gain_mana, grant_random_keywords, random_pool_keywords, tune_once,
};
use jackioh_engine::layers::unit_view;
use jackioh_engine::query::lucky_on;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::evolution::{
    EV_PING, EVOLVER, PINGER, PLAIN as EV_PLAIN, TWICE, register_evolution_fixtures,
};
use crate::rules::fixtures::harness::{apply_now, both, by, json_of, live};

/// A (0) 1/5 Unit with no text: libraries and hands, so no turn ends by itself.
const PLAIN: &str = "fvh-kw-plain";
/// A (0) 1/5 Unit with no text, to attack with.
const POKER: &str = "fvh-kw-poker";
/// A (0) 0/10 Unit with Fragile.
const FRAGILE: &str = "fvh-kw-fragile";
/// `FRAGILE` with Divine Shield.
const SHIELDED: &str = "fvh-kw-shielded";
/// `FRAGILE` with Indestructible.
const WARDED: &str = "fvh-kw-warded";
/// `FRAGILE` with Armor 2.
const ARMORED: &str = "fvh-kw-armored";
/// A (0) 0/10 Unit with no text.
const STURDY: &str = "fvh-kw-sturdy";
/// `FRAGILE` with every keyword of R21's pool besides, so a Buff's keyword row holds little but Fragile.
const PACKED: &str = "fvh-kw-packed";
/// 0/10 Units printing Lucky and Unlucky: (1, 0), (1, 1), (1, 2), (0, 2) and (3, 1).
const LUCKY: &str = "fvh-kw-lucky";
const JINXED: &str = "fvh-kw-jinxed";
const CURSED: &str = "fvh-kw-cursed";
const DOOMED: &str = "fvh-kw-doomed";
const BLESSED: &str = "fvh-kw-blessed";
/// A 0/10 Unit: Durability 3, Fragile, Unlucky 2, "Activate: gain 1 mana".
const SHOWN: &str = "fvh-kw-shown";
/// A Field Spell: Durability 2, "Activate: gain 1 mana".
const SHOWN_FIELD: &str = "fvh-kw-shown-field";
/// A (0) 2/2 Unit with Endless Evolve: "Evolve: deal 1 damage to the enemy hero."
const ENDLESS: &str = "fvh-kw-endless";
/// A (0) 2/2 Unit with Endless Evolve: "Evolves when you play a card. Evolve: deal 1 damage to the enemy
/// hero."
const ENDLESS_WATCH: &str = "fvh-kw-endless-watch";
/// `ENDLESS_WATCH` without Endless Evolve.
const WATCH: &str = "fvh-kw-watch";

const UNIT_ATTACK: i32 = 1;
const UNIT_HEALTH: i32 = 5;
const WALL_HEALTH: i32 = 10;
const EVOLVER_STATS: (i32, i32) = (2, 2);
const ARMOR: i32 = 2;
const SHOWN_DURABILITY: i32 = 3;
const SHOWN_UNLUCKY: i32 = 2;
const FIELD_DURABILITY: i32 = 2;
const GAIN: i32 = 1;
/// The fixtures' Lucky and Unlucky, as (Lucky, Unlucky).
const LUCKY_LUCK: (i32, i32) = (1, 0);
const JINXED_LUCK: (i32, i32) = (1, 1);
const CURSED_LUCK: (i32, i32) = (1, 2);
const DOOMED_LUCK: (i32, i32) = (0, 2);
const BLESSED_LUCK: (i32, i32) = (3, 1);
/// A hit larger than a Divine Shield needs and than `ARMOR` soaks, but below a wall's Health.
const BIG_HIT: i32 = 5;
/// How many times the Endless Evolve test evolves its Unit.
const EVOLVES: i32 = 3;
/// How many rng cursors a Buff or a Nerf is drawn at (`after_one`).
const SAMPLES: u32 = 60;
/// How many Nerfs in a row the never-removes tests apply.
const NERFS_IN_A_ROW: usize = 40;
/// The rng tests' seed, cursors and roll.
const RNG_SEED: &str = "r5236020-rng";
const TRIALS: u32 = 200;
const ROLL_RANGE: i32 = 1000;
/// A net luck of −2: two extra rolls, the worst kept.
const NET_UNLUCKY: i32 = -2;
const NET_LUCKY: i32 = 2;

// Fixtures

fn card(index: u32, id: &str, type_: &str, stats: (i32, i32), printed: Value, more: Vec<Keyword>) -> CardDef {
    let (attack, health) = stats;
    let is_unit = type_ == "Unit";
    let face = |scale: i32| {
        if is_unit {
            json!({
                "attack": attack * scale,
                "health": health * scale,
                "keywords": printed.clone(),
                "text": id,
            })
        } else {
            json!({ "keywords": printed.clone(), "text": id })
        }
    };
    let mut def: CardDef = json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (FvH keyword fixture)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face(1),
        "radiant": face(2),
    }));
    def.base.keywords.extend(more.iter().cloned());
    def.radiant.keywords.extend(more);
    def
}

fn wall(index: u32, id: &str, printed: Value, more: Vec<Keyword>) -> CardDef {
    card(index, id, "Unit", (0, WALL_HEALTH), printed, more)
}

fn lucky(n: i32) -> Value {
    json!([{ "kind": "Lucky", "n": n }])
}

fn luck(index: u32, id: &str, (lucky_n, unlucky_n): (i32, i32)) -> CardDef {
    let printed = if lucky_n > 0 { lucky(lucky_n) } else { json!([]) };
    let more = if unlucky_n > 0 {
        vec![Keyword::Unlucky { n: unlucky_n }]
    } else {
        vec![]
    };
    wall(index, id, printed, more)
}

fn defs() -> Vec<CardDef> {
    let unit = (UNIT_ATTACK, UNIT_HEALTH);
    vec![
        card(5236200, PLAIN, "Unit", unit, json!([]), vec![]),
        card(5236201, POKER, "Unit", unit, json!([]), vec![]),
        wall(5236202, FRAGILE, json!([]), vec![Keyword::Fragile]),
        wall(
            5236203,
            SHIELDED,
            json!([{ "kind": "Divine Shield" }]),
            vec![Keyword::Fragile],
        ),
        wall(
            5236204,
            WARDED,
            json!([{ "kind": "Indestructible" }]),
            vec![Keyword::Fragile],
        ),
        wall(
            5236205,
            ARMORED,
            json!([{ "kind": "Armor", "n": ARMOR }]),
            vec![Keyword::Fragile],
        ),
        wall(5236206, STURDY, json!([]), vec![]),
        wall(
            5236207,
            PACKED,
            json_of(random_pool_keywords()),
            vec![Keyword::Fragile],
        ),
        luck(5236208, LUCKY, LUCKY_LUCK),
        luck(5236209, JINXED, JINXED_LUCK),
        luck(5236210, CURSED, CURSED_LUCK),
        luck(5236211, DOOMED, DOOMED_LUCK),
        luck(5236212, BLESSED, BLESSED_LUCK),
        wall(
            5236213,
            SHOWN,
            json!([]),
            vec![
                Keyword::Durability { n: SHOWN_DURABILITY },
                Keyword::Fragile,
                Keyword::Unlucky { n: SHOWN_UNLUCKY },
            ],
        ),
        card(
            5236214,
            SHOWN_FIELD,
            "Field Spell",
            (0, 0),
            json!([]),
            vec![Keyword::Durability { n: FIELD_DURABILITY }],
        ),
        card(5236215, ENDLESS, "Unit", EVOLVER_STATS, json!([]), vec![]),
        card(5236216, ENDLESS_WATCH, "Unit", EVOLVER_STATS, json!([]), vec![]),
        card(5236217, WATCH, "Unit", EVOLVER_STATS, json!([]), vec![]),
    ]
}

fn ping() -> Effect {
    damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": EV_PING })))
}

/// "Evolve: ping", with Endless Evolve when `endless`, and "Evolves when you play a card" when `watches`.
fn evolving(endless: bool, watches: bool) -> Script {
    let triggers = if watches {
        vec![TriggerDef::new(
            "fvh-evolves-on-a-play",
            &[GameEventType::CardPlayed],
            move |ctx, event| {
                let GameEvent::CardPlayed { player, .. } = event else {
                    return vec![];
                };
                if *player == ctx.controller {
                    vec![evolve(json_as(json!({ "target": { "of": "self" } })))]
                } else {
                    vec![]
                }
            },
        )]
    } else {
        vec![]
    };
    Script {
        evolve: Some(hook(|_ctx| vec![ping()])),
        static_flags: endless.then(|| StaticFlags {
            endless_evolve: Some(true),
            ..StaticFlags::default()
        }),
        triggers,
        ..Script::default()
    }
}

fn scripts() -> IndexMap<String, CardScripts> {
    let gain = ActivationDecl {
        id: "gain".to_string(),
        label: "gain".to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|_ctx| vec![gain_mana(json_as(json!({ "amount": GAIN })))]),
    };
    let mut all: IndexMap<String, CardScripts> = IndexMap::new();
    for id in [SHOWN, SHOWN_FIELD] {
        all.insert(
            id.to_string(),
            both(Script {
                activations: vec![gain.clone()],
                ..Script::default()
            }),
        );
    }
    all.insert(ENDLESS.to_string(), both(evolving(true, false)));
    all.insert(ENDLESS_WATCH.to_string(), both(evolving(true, true)));
    all.insert(WATCH.to_string(), both(evolving(false, true)));
    all
}

/// The evolution fixtures (`fixtures::evolution`, over the engine's fixture catalog) with this file's
/// cards and scripts on top.
fn register() {
    register_evolution_fixtures();
    let mut catalog: CardDefs = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(scripts());
    register_scripts(merged);
}

/// p1's turn 9, its main phase; each side's setup as given, plain cards to draw and to hold, and no
/// turn that ends by itself (R345).
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    register();
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([PLAIN, PLAIN, PLAIN, PLAIN]);
        }
        if side.get("hand").is_none() {
            side["hand"] = json!([PLAIN]);
        }
    }
    let mut s = scenario(json!({ "seed": seed, "active": "p1", "p1": p1, "p2": p2 }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

/// A field entry in Attack Position, so no Defense Position lends it Armor.
fn attacking(def: &str) -> Value {
    json!({ "def": def, "position": "ATK" })
}

/// A field entry in Attack Position that carries `counters` (`{ "burn": 1 }`).
fn attacking_with(def: &str, counters: Value) -> Value {
    json!({ "def": def, "position": "ATK", "counters": counters })
}

// Reads and steps

fn hero_health(s: &Scenario, player: PlayerId) -> i32 {
    s.state().players[player].hero.health
}

fn kinds(keywords: &[Keyword]) -> Vec<KeywordKind> {
    keywords.iter().map(Keyword::kind).collect()
}

fn has_fragile(state: &GameState, card: &str) -> bool {
    unit_view(state, &live(state, card))
        .keywords
        .contains(&Keyword::Fragile)
}

/// How many Fragile the Unit shows (`unit_view`).
fn fragile_count(state: &GameState, card: &str) -> usize {
    unit_view(state, &live(state, card))
        .keywords
        .iter()
        .filter(|keyword| **keyword == Keyword::Fragile)
        .count()
}

/// The Unlucky the Unit shows (`unit_view`), 0 for none.
fn unlucky_of(state: &GameState, card: &str) -> i32 {
    unit_view(state, &live(state, card))
        .keywords
        .iter()
        .map(|keyword| match keyword {
            Keyword::Unlucky { n } => *n,
            _ => 0,
        })
        .sum()
}

/// The `via` of the `destroyed` event of `card` among `events`, or `None` when it did not die.
fn death_via(events: &[GameEvent], card: &str) -> Option<Option<KeywordKind>> {
    events.iter().find_map(|event| match event {
        GameEvent::Destroyed { instance_id, via, .. } if instance_id == card => Some(*via),
        _ => None,
    })
}

/// A hit of `amount` on `card` with no card behind it (no source), then the settle a resolving effect
/// ends with; its events.
fn hit(s: &mut Scenario, card: &str, amount: i32) -> Vec<GameEvent> {
    let state = s.state_mut();
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        {
            let mut ctx = make_context(&mut sink, None, by(P1));
            let effect = damage(json_as(json!({
                "to": { "of": "instance", "instanceId": card },
                "amount": amount,
            })));
            (effect.apply)(&mut ctx);
        }
        settle(&mut sink, SettleOptions::default());
    }
    state.rng_cursor = rng.cursor();
    events
}

/// One Nerf or Buff applied to `card` for p1, as R386's draw makes it, the rng cursor written back; its
/// events.
fn tuned_once(state: &mut GameState, card: &str, direction: TuneDirection) -> Vec<GameEvent> {
    let target = live(state, card);
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(&mut sink, None, by(P1));
        tune_once(&mut ctx, &target, direction, true);
    }
    state.rng_cursor = rng.cursor();
    events
}

/// `read` of the card after one Nerf or Buff drawn at each of `SAMPLES` rng cursors, each from the
/// scenario's state as it stands.
fn after_one<T>(
    s: &Scenario,
    card: &str,
    direction: TuneDirection,
    read: impl Fn(&GameState, &str) -> T,
) -> Vec<T> {
    (0..SAMPLES)
        .map(|cursor| {
            let mut state = s.state().clone();
            state.rng_cursor = cursor;
            tuned_once(&mut state, card, direction);
            read(&state, card)
        })
        .collect()
}

/// Whether `events` hold a Nerf or Buff that took Fragile off.
fn took_fragile_off(events: &[GameEvent]) -> bool {
    events.iter().any(|event| match event {
        GameEvent::Degraded { change, .. } | GameEvent::Upgraded { change, .. } => matches!(
            change,
            TuningChange::Keyword { keyword: Keyword::Fragile, added: false, .. }
        ),
        _ => false,
    })
}

mod r5236010_fragile_dies_to_any_hit_that_lowers_its_health {
    use super::*;

    #[test]
    fn r5236010_a_hit_from_an_attack_destroys_it() {
        let mut s = board(
            "r5236010-attack",
            json!({ "field": [attacking(POKER)] }),
            json!({ "field": [attacking(FRAGILE)] }),
        );
        let poker = s.unit_at(P1, 1).id;
        let fragile = s.unit_at(P2, 1).id;
        s.attack(poker.as_str(), fragile.as_str());
        s.expect_in_zone(fragile.as_str(), "graveyard");
        assert_eq!(
            death_via(s.last_events(), &fragile),
            Some(Some(KeywordKind::Fragile))
        );
        s.expect_in_zone(poker.as_str(), "field");
    }

    #[test]
    fn r5236010_a_hit_with_no_source_destroys_it() {
        let mut s = board(
            "r5236010-sourceless",
            json!({}),
            json!({ "field": [attacking(FRAGILE)] }),
        );
        let fragile = s.unit_at(P2, 1).id;
        let events = hit(&mut s, &fragile, 1);
        s.expect_in_zone(fragile.as_str(), "graveyard");
        assert_eq!(death_via(&events, &fragile), Some(Some(KeywordKind::Fragile)));
    }

    #[test]
    fn r5236010_a_burn_tick_destroys_it() {
        let mut s = board(
            "r5236010-burn",
            json!({ "field": [attacking_with(FRAGILE, json!({ "burn": 1 }))] }),
            json!({}),
        );
        let fragile = s.unit_at(P1, 1).id;
        s.end_turn();
        s.expect_in_zone(fragile.as_str(), "graveyard");
    }

    #[test]
    fn r5236010_a_toxic_tick_destroys_it() {
        let mut s = board(
            "r5236010-toxic",
            json!({}),
            json!({ "field": [attacking_with(FRAGILE, json!({ "toxic": 1 }))] }),
        );
        let fragile = s.unit_at(P2, 1).id;
        // p2's turn starts, and its Toxic ticks.
        s.end_turn();
        s.expect_in_zone(fragile.as_str(), "graveyard");
    }

    #[test]
    fn r5236010_a_hit_its_divine_shield_takes_deals_0_and_leaves_it_standing() {
        let mut s = board(
            "r5236010-shield",
            json!({}),
            json!({ "field": [attacking(SHIELDED)] }),
        );
        let fragile = s.unit_at(P2, 1).id;
        let events = hit(&mut s, &fragile, BIG_HIT);
        s.expect_in_zone(fragile.as_str(), "field");
        assert_eq!(s.card(fragile.as_str()).damage, 0);
        assert_eq!(death_via(&events, &fragile), None);
        // The shield gone, the next hit lowers its Health.
        hit(&mut s, &fragile, 1);
        s.expect_in_zone(fragile.as_str(), "graveyard");
    }

    #[test]
    fn r5236010_indestructible_still_saves_it() {
        let mut s = board(
            "r5236010-warded",
            json!({}),
            json!({ "field": [attacking(WARDED)] }),
        );
        let fragile = s.unit_at(P2, 1).id;
        let events = hit(&mut s, &fragile, 1);
        s.expect_in_zone(fragile.as_str(), "field");
        s.expect_stats(fragile.as_str(), json!({ "health": WALL_HEALTH - 1 }));
        assert_eq!(death_via(&events, &fragile), None);
    }

    #[test]
    fn r5236010_armor_that_soaks_a_whole_hit_leaves_it_standing() {
        let mut s = board(
            "r5236010-armor",
            json!({}),
            json!({ "field": [attacking(ARMORED)] }),
        );
        let fragile = s.unit_at(P2, 1).id;
        hit(&mut s, &fragile, ARMOR);
        s.expect_in_zone(fragile.as_str(), "field");
        assert_eq!(s.card(fragile.as_str()).damage, 0);
        // A hit Armor takes only part of lowers its Health.
        hit(&mut s, &fragile, BIG_HIT);
        s.expect_in_zone(fragile.as_str(), "graveyard");
    }

    #[test]
    fn r5236010_a_hit_of_0_leaves_it_standing() {
        let mut s = board(
            "r5236010-zero",
            json!({}),
            json!({ "field": [attacking(FRAGILE)] }),
        );
        let fragile = s.unit_at(P2, 1).id;
        let events = hit(&mut s, &fragile, 0);
        s.expect_in_zone(fragile.as_str(), "field");
        assert_eq!(death_via(&events, &fragile), None);
    }

    #[test]
    fn r5236010_a_unit_without_it_survives_the_same_hit() {
        let mut s = board(
            "r5236010-sturdy",
            json!({}),
            json!({ "field": [attacking(STURDY)] }),
        );
        let sturdy = s.unit_at(P2, 1).id;
        let events = hit(&mut s, &sturdy, 1);
        s.expect_in_zone(sturdy.as_str(), "field");
        s.expect_stats(sturdy.as_str(), json!({ "health": WALL_HEALTH - 1 }));
        assert_eq!(death_via(&events, &sturdy), None);
    }
}

mod r5236011_fragile_under_a_nerf_and_a_buff {
    use super::*;

    #[test]
    fn r5236011_fragile_is_not_in_r21s_pool() {
        let pool = json_of(RANDOM_KEYWORD_POOL);
        assert!(
            pool.as_array()
                .is_some_and(|names| !names.contains(&json!("Fragile"))),
            "{pool}"
        );
        assert!(!random_pool_keywords().contains(&Keyword::Fragile));
    }

    #[test]
    fn r5236011_a_grant_of_the_whole_pool_never_gives_fragile() {
        let mut s = board("r5236011-grant", json!({ "field": [attacking(STURDY)] }), json!({}));
        let sturdy = s.unit_at(P1, 1).id;
        apply_now(
            s.state_mut(),
            grant_random_keywords(json_as(json!({
                "target": { "of": "instance", "instanceId": sturdy },
                "count": RANDOM_KEYWORD_POOL.len(),
            }))),
        );
        assert!(!s.card(sturdy.as_str()).granted_keywords.is_empty(), "setup: the grant landed");
        assert!(!has_fragile(s.state(), &sturdy));
    }

    #[test]
    fn r5236011_a_nerf_may_add_fragile_to_a_unit() {
        let s = board("r5236011-nerf-adds", json!({ "field": [attacking(STURDY)] }), json!({}));
        let sturdy = s.unit_at(P1, 1).id;
        assert!(!has_fragile(s.state(), &sturdy));
        let after = after_one(&s, &sturdy, TuneDirection::Degrade, has_fragile);
        assert!(after.contains(&true), "no Nerf of {SAMPLES} added Fragile");
    }

    #[test]
    fn r5236011_a_nerf_never_removes_fragile() {
        let s = board("r5236011-nerf-keeps", json!({ "field": [attacking(FRAGILE)] }), json!({}));
        let fragile = s.unit_at(P1, 1).id;
        let after = after_one(&s, &fragile, TuneDirection::Degrade, has_fragile);
        assert!(after.iter().all(|kept| *kept), "{after:?}");
        let mut state = s.state().clone();
        for _ in 0..NERFS_IN_A_ROW {
            let events = tuned_once(&mut state, &fragile, TuneDirection::Degrade);
            assert!(!took_fragile_off(&events), "{events:?}");
            assert!(has_fragile(&state, &fragile));
        }
    }

    #[test]
    fn r5236011_a_nerf_never_gives_a_fragile_unit_a_second_fragile() {
        let s = board("r5236011-nerf-once", json!({ "field": [attacking(FRAGILE)] }), json!({}));
        let fragile = s.unit_at(P1, 1).id;
        let after = after_one(&s, &fragile, TuneDirection::Degrade, fragile_count);
        assert!(after.iter().all(|count| *count == 1), "{after:?}");
    }

    #[test]
    fn r5236011_a_buff_removes_fragile() {
        let s = board("r5236011-buff-removes", json!({ "field": [attacking(PACKED)] }), json!({}));
        let packed = s.unit_at(P1, 1).id;
        assert!(has_fragile(s.state(), &packed));
        let after = after_one(&s, &packed, TuneDirection::Upgrade, has_fragile);
        assert!(after.contains(&false), "no Buff of {SAMPLES} took Fragile off");
    }

    #[test]
    fn r5236011_a_buff_never_adds_fragile() {
        let s = board("r5236011-buff-adds", json!({ "field": [attacking(STURDY)] }), json!({}));
        let sturdy = s.unit_at(P1, 1).id;
        let after = after_one(&s, &sturdy, TuneDirection::Upgrade, has_fragile);
        assert!(after.iter().all(|added| !*added), "{after:?}");
    }
}

mod r5236020_net_luck_and_the_worst_of_roll {
    use super::*;

    /// `count` rolls of `ROLL_RANGE` from `cursor`, and the cursor they leave.
    fn rolls(cursor: u32, count: usize) -> (Vec<i32>, u32) {
        let mut rng = Rng::new(RNG_SEED, cursor);
        let drawn: Vec<i32> = (0..count).map(|_| rng.int(ROLL_RANGE)).collect();
        (drawn, rng.cursor())
    }

    fn coins(cursor: u32, count: usize) -> (Vec<bool>, u32) {
        let mut rng = Rng::new(RNG_SEED, cursor);
        let flipped: Vec<bool> = (0..count).map(|_| rng.coin()).collect();
        (flipped, rng.cursor())
    }

    /// 1 + |net| rolls for a net of `net`.
    fn rolls_for(net: i32) -> usize {
        1 + net.unsigned_abs() as usize
    }

    #[test]
    fn r5236020_net_luck_is_lucky_minus_unlucky() {
        let s = board(
            "r5236020-net",
            json!({ "field": [LUCKY, JINXED, CURSED, DOOMED, BLESSED] }),
            json!({}),
        );
        let net = |lane: i32| lucky_on(s.state(), &s.unit_at(P1, lane));
        let expected = |(lucky_n, unlucky_n): (i32, i32)| lucky_n - unlucky_n;
        assert_eq!(
            [net(1), net(2), net(3), net(4), net(5)],
            [
                expected(LUCKY_LUCK),
                expected(JINXED_LUCK),
                expected(CURSED_LUCK),
                expected(DOOMED_LUCK),
                expected(BLESSED_LUCK),
            ]
        );
    }

    #[test]
    fn r5236020_a_card_with_neither_has_a_net_of_0() {
        let s = board("r5236020-none", json!({ "field": [STURDY] }), json!({}));
        assert_eq!(lucky_on(s.state(), &s.unit_at(P1, 1)), 0);
    }

    #[test]
    fn r5236020_a_negative_net_rolls_that_many_more_and_keeps_the_worst() {
        for cursor in 0..TRIALS {
            let (drawn, end) = rolls(cursor, rolls_for(NET_UNLUCKY));
            let mut rng = Rng::new(RNG_SEED, cursor);
            let higher_better = rng.lucky(NET_UNLUCKY, |rng| rng.int(ROLL_RANGE), Ord::max);
            assert_eq!(Some(&higher_better), drawn.iter().min(), "cursor {cursor}");
            assert_eq!(rng.cursor(), end, "1 + |net| draws, cursor {cursor}");
            // The worst is the comparator's: where lower is better, the highest is kept.
            let mut rng = Rng::new(RNG_SEED, cursor);
            let lower_better = rng.lucky(NET_UNLUCKY, |rng| rng.int(ROLL_RANGE), Ord::min);
            assert_eq!(Some(&lower_better), drawn.iter().max(), "cursor {cursor}");
        }
    }

    #[test]
    fn r5236020_a_coin_with_a_negative_net_keeps_tails_if_any() {
        let mut kept_tails_over_heads = false;
        for cursor in 0..TRIALS {
            let (flipped, end) = coins(cursor, rolls_for(NET_UNLUCKY));
            let mut rng = Rng::new(RNG_SEED, cursor);
            let heads = rng.lucky_coin(NET_UNLUCKY);
            assert_eq!(heads, flipped.iter().all(|flip| *flip), "cursor {cursor}: {flipped:?}");
            assert_eq!(rng.cursor(), end, "cursor {cursor}");
            kept_tails_over_heads |= !heads && flipped.contains(&true);
        }
        assert!(kept_tails_over_heads, "no trial mixed heads and tails");
    }

    #[test]
    fn r5236020_a_net_of_0_rolls_once() {
        for cursor in 0..TRIALS {
            let (drawn, end) = rolls(cursor, 1);
            let mut rng = Rng::new(RNG_SEED, cursor);
            assert_eq!(rng.lucky(0, |rng| rng.int(ROLL_RANGE), Ord::max), drawn[0]);
            assert_eq!(rng.cursor(), end);
            let (flipped, flip_end) = coins(cursor, 1);
            let mut rng = Rng::new(RNG_SEED, cursor);
            assert_eq!(rng.lucky_coin(0), flipped[0]);
            assert_eq!(rng.cursor(), flip_end);
        }
    }

    #[test]
    fn r5236020_a_positive_net_still_keeps_the_best() {
        for cursor in 0..TRIALS {
            let (drawn, end) = rolls(cursor, rolls_for(NET_LUCKY));
            let mut rng = Rng::new(RNG_SEED, cursor);
            let best = rng.lucky(NET_LUCKY, |rng| rng.int(ROLL_RANGE), Ord::max);
            assert_eq!(Some(&best), drawn.iter().max(), "cursor {cursor}");
            assert_eq!(rng.cursor(), end);
        }
    }
}

mod r5236021_unlucky_under_a_nerf_and_a_buff {
    use super::*;

    #[test]
    fn r5236021_a_nerf_adds_unlucky_1_to_a_card_with_lucky() {
        let s = board("r5236021-adds", json!({ "field": [LUCKY] }), json!({}));
        let id = s.unit_at(P1, 1).id;
        assert_eq!(unlucky_of(s.state(), &id), 0);
        let after = after_one(&s, &id, TuneDirection::Degrade, unlucky_of);
        assert!(after.contains(&1), "no Nerf of {SAMPLES} added Unlucky 1: {after:?}");
        assert!(after.iter().all(|n| *n <= 1), "a Nerf adds Unlucky 1, never more: {after:?}");
    }

    #[test]
    fn r5236021_a_nerf_raises_unlucky() {
        let s = board("r5236021-raises", json!({ "field": [JINXED] }), json!({}));
        let id = s.unit_at(P1, 1).id;
        let (_, before) = JINXED_LUCK;
        let after = after_one(&s, &id, TuneDirection::Degrade, unlucky_of);
        assert!(after.contains(&(before + 1)), "{after:?}");
        assert!(after.iter().all(|n| *n <= before + 1), "{after:?}");
    }

    #[test]
    fn r5236021_a_buff_lowers_unlucky() {
        let s = board("r5236021-lowers", json!({ "field": [CURSED] }), json!({}));
        let id = s.unit_at(P1, 1).id;
        let (_, before) = CURSED_LUCK;
        let after = after_one(&s, &id, TuneDirection::Upgrade, unlucky_of);
        assert!(after.contains(&(before - 1)), "{after:?}");
        assert!(after.iter().all(|n| *n <= before), "a Buff never raises it: {after:?}");
    }

    #[test]
    fn r5236021_a_buff_never_gives_unlucky() {
        let s = board("r5236021-buff-none", json!({ "field": [LUCKY] }), json!({}));
        let id = s.unit_at(P1, 1).id;
        let after = after_one(&s, &id, TuneDirection::Upgrade, unlucky_of);
        assert!(after.iter().all(|n| *n == 0), "{after:?}");
    }

    #[test]
    fn r5236021_a_nerf_never_gives_unlucky_to_a_card_with_no_luck_roll() {
        let s = board("r5236021-no-roll", json!({ "field": [attacking(STURDY)] }), json!({}));
        let sturdy = s.unit_at(P1, 1).id;
        let after = after_one(&s, &sturdy, TuneDirection::Degrade, unlucky_of);
        assert!(after.iter().all(|n| *n == 0), "{after:?}");
        let mut state = s.state().clone();
        for _ in 0..NERFS_IN_A_ROW {
            tuned_once(&mut state, &sturdy, TuneDirection::Degrade);
            assert_eq!(unlucky_of(&state, &sturdy), 0);
        }
    }
}

mod r5236030_endless_evolve {
    use super::*;

    fn evolved_count(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| event.event_type() == GameEventType::Evolved)
            .count()
    }

    #[test]
    fn r5236030_it_evolves_every_time_it_is_evolved() {
        let mut s = board(
            "r5236030-verb",
            json!({ "hand": [EVOLVER, EVOLVER, EVOLVER], "field": [ENDLESS] }),
            json!({}),
        );
        let id = s.unit_at(P1, 1).id;
        let health = hero_health(&s, P2);
        for _ in 0..EVOLVES {
            s.play(EVOLVER, pick::targets(pick::id(&id)));
        }
        s.expect_health(P2, health - EVOLVES * EV_PING);
        assert_eq!(query::evolutions_of(s.state(), P1), EVOLVES);
        assert_eq!(evolved_count(&s), EVOLVES as usize);
    }

    #[test]
    fn r5236030_without_it_a_second_evolve_does_nothing() {
        let mut s = board(
            "r5236030-once",
            json!({ "hand": [EVOLVER, EVOLVER], "field": [PINGER] }),
            json!({}),
        );
        let id = s.unit_at(P1, 1).id;
        let health = hero_health(&s, P2);
        s.play(EVOLVER, pick::targets(pick::id(&id)));
        s.play(EVOLVER, pick::targets(pick::id(&id)));
        s.expect_health(P2, health - EV_PING);
        assert_eq!(query::evolutions_of(s.state(), P1), 1);
        assert_eq!(evolved_count(&s), 1);
    }

    #[test]
    fn r5236030_it_evolves_again_each_time_its_trigger_condition_is_met() {
        let mut s = board(
            "r5236030-watch",
            json!({ "hand": [EV_PLAIN, EV_PLAIN], "field": [{ "def": ENDLESS_WATCH, "lane": 1 }] }),
            json!({}),
        );
        let health = hero_health(&s, P2);
        s.play(EV_PLAIN, json!({ "zone": 2 }));
        s.play(EV_PLAIN, json!({ "zone": 3 }));
        s.expect_health(P2, health - 2 * EV_PING);
        assert_eq!(query::evolutions_of(s.state(), P1), 2);
    }

    #[test]
    fn r5236030_a_watcher_without_it_evolves_once() {
        let mut s = board(
            "r5236030-watch-once",
            json!({ "hand": [EV_PLAIN, EV_PLAIN], "field": [{ "def": WATCH, "lane": 1 }] }),
            json!({}),
        );
        let health = hero_health(&s, P2);
        s.play(EV_PLAIN, json!({ "zone": 2 }));
        s.play(EV_PLAIN, json!({ "zone": 3 }));
        s.expect_health(P2, health - EV_PING);
        assert_eq!(query::evolutions_of(s.state(), P1), 1);
    }

    #[test]
    fn r5236030_the_multiplier_holds_for_each_evolution() {
        let mut s = board(
            "r5236030-twice",
            json!({ "hand": [EVOLVER, EVOLVER], "field": [ENDLESS], "backrow": [TWICE] }),
            json!({}),
        );
        let id = s.unit_at(P1, 1).id;
        let health = hero_health(&s, P2);
        s.play(EVOLVER, pick::targets(pick::id(&id)));
        s.play(EVOLVER, pick::targets(pick::id(&id)));
        // Two evolutions, each running its text twice (R3536003), counted once each.
        s.expect_health(P2, health - 4 * EV_PING);
        assert_eq!(query::evolutions_of(s.state(), P1), 2);
    }
}

mod r5236031_the_instance_view_shows_them {
    use super::*;

    /// The keywords the Unit `card` shows in `view`, on the viewer's side or the other.
    fn unit_keywords(view: &PlayerView, own: bool, card: &str) -> Vec<Keyword> {
        let side = if own { &view.you } else { &view.opponent };
        side.units
            .iter()
            .flatten()
            .find(|unit| unit.instance_id == card)
            .map(|unit| unit.keywords.clone())
            .unwrap_or_else(|| panic!("the view shows no Unit {card}"))
    }

    /// The keywords the face-up backrow card `card` shows on the viewer's own side.
    fn backrow_keywords(view: &PlayerView, card: &str) -> Vec<Keyword> {
        view.you
            .backrow
            .iter()
            .flatten()
            .find_map(|entry| match entry {
                BackrowView::Public(shown) if shown.instance_id == card => {
                    Some(shown.keywords.clone().unwrap_or_default())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("the view shows no face-up backrow card {card}"))
    }

    fn durability_shown(keywords: &[Keyword]) -> Vec<i32> {
        keywords
            .iter()
            .filter_map(|keyword| match keyword {
                Keyword::Durability { n } => Some(*n),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r5236031_a_units_view_shows_durability_with_the_count_left() {
        let mut s = board("r5236031-unit", json!({ "field": [SHOWN] }), json!({}));
        let id = s.unit_at(P1, 1).id;
        assert_eq!(
            durability_shown(&unit_keywords(&s.view(P1), true, &id)),
            vec![SHOWN_DURABILITY]
        );
        s.activate(id.as_str(), json!({}));
        assert_eq!(
            durability_shown(&unit_keywords(&s.view(P1), true, &id)),
            vec![SHOWN_DURABILITY - 1],
            "the count left, not the printed number"
        );
        assert_eq!(
            durability_shown(&unit_keywords(&s.view(P2), false, &id)),
            vec![SHOWN_DURABILITY - 1]
        );
    }

    #[test]
    fn r5236031_a_units_view_shows_fragile_and_unlucky_to_both_seats() {
        let s = board("r5236031-fragile", json!({ "field": [SHOWN] }), json!({}));
        let id = s.unit_at(P1, 1).id;
        for (viewer, own) in [(P1, true), (P2, false)] {
            let keywords = unit_keywords(&s.view(viewer), own, &id);
            assert!(keywords.contains(&Keyword::Fragile), "{viewer:?}: {keywords:?}");
            assert!(
                keywords.contains(&Keyword::Unlucky { n: SHOWN_UNLUCKY }),
                "{viewer:?}: {keywords:?}"
            );
        }
    }

    #[test]
    fn r5236031_a_field_spells_view_shows_its_durability_left() {
        let mut s = board(
            "r5236031-field",
            json!({ "backrow": [{ "def": SHOWN_FIELD, "faceUp": true }] }),
            json!({}),
        );
        let id = s
            .backrow(P1, 1)
            .unwrap_or_else(|| panic!("setup: no Field Spell in p1's backrow lane 1"))
            .id;
        assert_eq!(
            durability_shown(&backrow_keywords(&s.view(P1), &id)),
            vec![FIELD_DURABILITY]
        );
        s.activate(id.as_str(), json!({}));
        assert_eq!(
            durability_shown(&backrow_keywords(&s.view(P1), &id)),
            vec![FIELD_DURABILITY - 1]
        );
    }

    #[test]
    fn r5236031_a_unit_without_them_shows_none_of_them() {
        let s = board("r5236031-none", json!({ "field": [STURDY] }), json!({}));
        let id = s.unit_at(P1, 1).id;
        let shown = kinds(&unit_keywords(&s.view(P1), true, &id));
        for kind in [KeywordKind::Durability, KeywordKind::Fragile, KeywordKind::Unlucky] {
            assert!(!shown.contains(&kind), "{shown:?}");
        }
    }
}
