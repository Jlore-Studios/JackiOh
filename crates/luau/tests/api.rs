//! The card API's tests (#448): one test per hook adapter over fixture scripts,
//! and Luau-vs-Rust parity per verb group — the same card as a Luau fixture
//! and as an engine fixture, played in identical scenarios, with equal
//! `hash_state` and equal events.

use std::sync::Arc;

use jackioh_engine::testkit::{
    CardDefs, EffectContext, IndexMap, json_as, register_catalog, register_scripts, scenario,
};
use jackioh_engine::{CardScripts, Script, hook};
use serde_json::{Value, json};

use jackioh_luau::api::{all_verbs, effect_of};
use jackioh_luau::{EffectCall, Face, Site, compile, load_card, run_hook};

/// A source compiled (and linted) to bytecode that lives as long as the test run,
/// as a card's `include_bytes!` will. Compile-time reads only: rule 4 bans the disk
/// in this crate, tests included.
fn bytecode(file: &str, source: &str) -> &'static [u8] {
    match compile(file, source) {
        Ok(bytecode) => Vec::leak(bytecode),
        Err(error) => panic!("{error}"),
    }
}

/// `fixtures/<name>.luau`, compiled.
macro_rules! fixture {
    ($name:literal) => {
        bytecode(
            concat!($name, ".luau"),
            include_str!(concat!("fixtures/", $name, ".luau")),
        )
    };
}

mod common {
    use super::*;

    /// A 0-cost fixture Spell.
    pub fn spell_def(id: &str) -> jackioh_engine::CardDef {
        json_as(json!({
            "id": id, "index": "lx-000", "name": id,
            "set": "Core", "type": "Spell", "tags": [],
            "rarity": "Common", "token": false, "cost": 0,
            "base": { "keywords": [], "text": "test fixture" },
            "radiant": { "keywords": [], "text": "test fixture" },
        }))
    }

    /// A 0-cost vanilla fixture Unit.
    pub fn unit_def(id: &str, attack: i32, health: i32) -> jackioh_engine::CardDef {
        json_as(json!({
            "id": id, "index": "lx-000", "name": id,
            "set": "Core", "type": "Unit", "tags": [],
            "rarity": "Common", "token": false, "cost": 0,
            "base": { "attack": attack, "health": health, "keywords": [], "text": "vanilla" },
            "radiant": { "attack": attack, "health": health, "keywords": [], "text": "vanilla" },
        }))
    }

    /// The catalog every test shares: the parity defs plus filler vanilla
    /// units, so default decks build. The override is thread-local (like
    /// `host.rs`), so this runs per thread, not once. Scripts are registered
    /// per run by `check_parity`, never here.
    pub fn ensure_installed() {
        use jackioh_engine::testkit::catalog_override;
        if catalog_override().is_some() {
            return;
        }
        let mut defs: CardDefs = CardDefs::new();
        for id in [
            "lx-p-damage",
            "lx-p-destroy",
            "lx-p-draw",
            "lx-p-buff",
            "lx-p-summon",
            "lx-p-mana",
            "lx-p-exile",
            "lx-p-heal",
            "lx-p-discard",
            "lx-p-after-check",
            "lx-p-for-each",
            "lx-p-draw-while",
            "lx-p-bad-hook",
        ] {
            defs.insert(id.to_string(), spell_def(id));
        }
        defs.insert("lx-p-morph".to_string(), unit_def("lx-p-morph", 2, 2));
        defs.insert("lx-fx-001".to_string(), unit_def("lx-fx-001", 2, 2));
        defs.insert("lx-fx-002".to_string(), unit_def("lx-fx-002", 3, 3));
        for n in 0..30 {
            let id = format!("lx-filler-{n:02}");
            defs.insert(id.clone(), unit_def(&id, 1, 1));
        }
        register_catalog(defs);
    }

    /// Plays `id` in a fixed-seed scenario and returns the state's hash and
    /// its events as JSON.
    pub fn run_card(id: &str, opts: Value, play: Value) -> (String, Value) {
        let mut opts = opts;
        opts["seed"] = Value::String("lx-parity".to_string());
        let mut s = scenario(opts);
        s.play(id, play);
        let hash = jackioh_engine::replay::hash_state(s.state());
        let events = serde_json::to_value(s.events()).unwrap();
        (hash, events)
    }

    /// The same card as a Rust script and as a Luau fixture, under one card
    /// id so states and events compare exactly: equal `hash_state` and equal
    /// events. The registry is replaced per run, so the two runs share
    /// nothing but the id.
    pub fn check_parity(
        id: &'static str,
        rust: Script,
        luau_code: &'static [u8],
        mk: impl Fn() -> Value,
        play: Value,
    ) {
        ensure_installed();
        let mut only: IndexMap<String, CardScripts> = IndexMap::new();
        only.insert(
            id.to_string(),
            CardScripts {
                base: rust.clone(),
                radiant: rust,
            },
        );
        register_scripts(only);
        let rust_out = run_card(id, mk(), play.clone());
        let luau = load_card(id, luau_code).expect("luau twin loads");
        let mut only: IndexMap<String, CardScripts> = IndexMap::new();
        only.insert(id.to_string(), luau);
        register_scripts(only);
        let luau_out = run_card(id, mk(), play);
        assert_eq!(rust_out.0, luau_out.0, "hash_state differs for {id}");
        assert_eq!(rust_out.1, luau_out.1, "events differ for {id}");
    }

    /// Default scenario: the named card alone in P1's hand, filler elsewhere.
    pub fn hand_of(id: &str) -> Value {
        json!({ "p1": { "hand": [id] } })
    }
}

// --------------------------------------------------------------------------------
// Verb parity: the same card in Rust and in Luau, played in identical scenarios.
// --------------------------------------------------------------------------------

#[test]
fn parity_damage() {
    use jackioh_engine::effects::damage;
    let rust = Script {
        cry: Some(hook(|_| {
            vec![damage(json_as(
                json!({ "to": { "of": "enemyHero" }, "amount": 3 }),
            ))]
        })),
        ..Default::default()
    };
    common::check_parity(
        "lx-p-damage",
        rust,
        fixture!("parity_damage"),
        || common::hand_of("lx-p-damage"),
        json!({}),
    );
}

#[test]
fn parity_destroy() {
    use jackioh_engine::effects::destroy_all;
    let rust = Script {
        cry: Some(hook(|_| vec![destroy_all(json_as(json!({ "side": "enemy" })))])),
        ..Default::default()
    };
    let mk = || json!({ "p1": { "hand": ["lx-p-destroy"] }, "p2": { "field": ["lx-fx-001"] } });
    common::check_parity("lx-p-destroy", rust, fixture!("parity_destroy"), mk, json!({}));
}

#[test]
fn parity_draw() {
    use jackioh_engine::effects::draw;
    let rust = Script {
        cry: Some(hook(|_| vec![draw(json_as(json!({ "count": 2 })))])),
        ..Default::default()
    };
    let mk = || {
        json!({ "p1": {
            "hand": ["lx-p-draw"],
            "library": ["lx-filler-00", "lx-filler-01", "lx-filler-02"],
        } })
    };
    common::check_parity("lx-p-draw", rust, fixture!("parity_draw"), mk, json!({}));
}

#[test]
fn parity_buff() {
    use jackioh_engine::effects::buff_all_units;
    let rust = Script {
        cry: Some(hook(|_| {
            vec![buff_all_units(json_as(json!({ "attack": 1, "health": 1 })))]
        })),
        ..Default::default()
    };
    let mk = || json!({ "p1": { "hand": ["lx-p-buff"], "field": ["lx-fx-001"] } });
    common::check_parity("lx-p-buff", rust, fixture!("parity_buff"), mk, json!({}));
}

#[test]
fn parity_summon() {
    use jackioh_engine::effects::summon;
    let rust = Script {
        cry: Some(hook(|_| vec![summon(json_as(json!({ "defId": "lx-fx-001" })))])),
        ..Default::default()
    };
    common::check_parity(
        "lx-p-summon",
        rust,
        fixture!("parity_summon"),
        || common::hand_of("lx-p-summon"),
        json!({}),
    );
}

#[test]
fn parity_mana() {
    use jackioh_engine::effects::gain_mana;
    let rust = Script {
        cry: Some(hook(|_| vec![gain_mana(json_as(json!({ "amount": 3 })))])),
        ..Default::default()
    };
    common::check_parity(
        "lx-p-mana",
        rust,
        fixture!("parity_mana"),
        || common::hand_of("lx-p-mana"),
        json!({}),
    );
}

#[test]
fn parity_exile() {
    use jackioh_engine::effects::exile_all;
    let rust = Script {
        cry: Some(hook(|_| vec![exile_all(json_as(json!({ "side": "enemy" })))])),
        ..Default::default()
    };
    let mk = || json!({ "p1": { "hand": ["lx-p-exile"] }, "p2": { "field": ["lx-fx-001"] } });
    common::check_parity("lx-p-exile", rust, fixture!("parity_exile"), mk, json!({}));
}

#[test]
fn parity_heal() {
    use jackioh_engine::effects::{damage, heal};
    let rust = Script {
        cry: Some(hook(|_| {
            vec![
                damage(json_as(json!({ "to": { "of": "selfHero" }, "amount": 5 }))),
                heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": 3 }))),
            ]
        })),
        ..Default::default()
    };
    common::check_parity(
        "lx-p-heal",
        rust,
        fixture!("parity_heal"),
        || common::hand_of("lx-p-heal"),
        json!({}),
    );
}

#[test]
fn parity_discard() {
    use jackioh_engine::effects::discard_random;
    let rust = Script {
        cry: Some(hook(|_| vec![discard_random(json_as(json!({ "count": 1 })))])),
        ..Default::default()
    };
    let mk = || json!({ "p1": { "hand": ["lx-p-discard", "lx-filler-00", "lx-filler-01", "lx-filler-02"] } });
    common::check_parity("lx-p-discard", rust, fixture!("parity_discard"), mk, json!({}));
}

#[test]
fn parity_transform() {
    use jackioh_engine::effects::transform;
    let rust = Script {
        cry: Some(hook(|_| {
            vec![transform(json_as(
                json!({ "target": { "of": "self" }, "defId": "lx-fx-002" }),
            ))]
        })),
        ..Default::default()
    };
    // Units play to a lane: the morph cries itself into lx-fx-002.
    common::check_parity(
        "lx-p-morph",
        rust,
        fixture!("parity_transform"),
        || common::hand_of("lx-p-morph"),
        json!({ "zone": 1 }),
    );
}

#[test]
fn parity_after_state_check() {
    use jackioh_engine::effects::{after_state_check, damage};
    // The one verb that takes a closure: a Luau function vs a Rust closure.
    let rust = Script {
        cry: Some(hook(|_| {
            vec![after_state_check(|_| {
                vec![damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": 2 }),
                ))]
            })]
        })),
        ..Default::default()
    };
    common::check_parity(
        "lx-p-after-check",
        rust,
        fixture!("parity_after_check"),
        || common::hand_of("lx-p-after-check"),
        json!({}),
    );
}

/// A verb naming a face function that is not there fails when the verb builds, with
/// an error naming the card, face and name — so a misspelling fails the card's own
/// test rather than firing nothing.
#[test]
#[should_panic(expected = "lx-p-bad-hook base cry: runtime error: after_chek: no such face function")]
fn misspelled_sub_hook_name_fails_naming_card_face_and_hook() {
    common::ensure_installed();
    let code = fixture!("bad_hook_name");
    let scripts = load_card("lx-p-bad-hook", code).expect("loads: names resolve at build");
    let mut only: IndexMap<String, CardScripts> = IndexMap::new();
    only.insert("lx-p-bad-hook".to_string(), scripts);
    register_scripts(only);
    let _ = common::run_card("lx-p-bad-hook", common::hand_of("lx-p-bad-hook"), json!({}));
}

#[test]
fn parity_for_each_card() {
    use jackioh_engine::effects::{ForEachCardArgs, damage, for_each_card};
    use jackioh_engine::{active_units_of, opponent_of};
    // Two `for_each_card`s in one hook, as Classic #39 holds: the Luau twin names
    // its second pair `each_cards_2` and `each_card_2`.
    fn one() -> jackioh_engine::Effect {
        for_each_card(ForEachCardArgs {
            cards: Arc::new(|ctx| {
                active_units_of(ctx.state, opponent_of(ctx.controller))
                    .iter()
                    .map(|unit| unit.id.clone())
                    .collect()
            }),
            each: Arc::new(|id| {
                damage(json_as(json!({
                    "to": { "of": "instance", "instanceId": id },
                    "amount": 1,
                })))
            }),
        })
    }
    let rust = Script {
        cry: Some(hook(|_| vec![one(), one()])),
        ..Default::default()
    };
    let mk = || json!({ "p1": { "hand": ["lx-p-for-each"] }, "p2": { "field": ["lx-fx-001"] } });
    common::check_parity("lx-p-for-each", rust, fixture!("parity_for_each"), mk, json!({}));
}

#[test]
fn parity_draw_while() {
    use jackioh_engine::effects::{DrawWhileArgs, draw_while};
    use jackioh_engine::{OffFieldZone, zone_cards};
    let rust = Script {
        cry: Some(hook(|_| {
            vec![draw_while(DrawWhileArgs {
                more: Arc::new(|ctx| zone_cards(ctx.state, ctx.controller, OffFieldZone::Hand).len() < 3),
                player: None,
            })]
        })),
        ..Default::default()
    };
    let mk = || {
        json!({ "p1": {
            "hand": ["lx-p-draw-while"],
            "library": ["lx-filler-00", "lx-filler-01", "lx-filler-02", "lx-filler-03", "lx-filler-04"],
        } })
    };
    common::check_parity(
        "lx-p-draw-while",
        rust,
        fixture!("parity_draw_while"),
        mk,
        json!({}),
    );
}

// --------------------------------------------------------------------------------
// Declarations: targets, modes, flags, activations, triggers, replacements,
// target checks, resume and grants load as data and run.
// --------------------------------------------------------------------------------

mod declarations {
    use super::*;
    use jackioh_engine::PlayerId;
    use jackioh_luau::hooks::*;
    use jackioh_luau::{Face, Site};

    fn site(hook: &'static str) -> Site {
        Site {
            card: "lx-triggers",
            face: Face::Base,
            hook,
        }
    }

    fn code() -> &'static [u8] {
        fixture!("triggers")
    }

    fn scripts() -> CardScripts {
        load_card("lx-triggers", code()).expect("triggers fixture loads")
    }

    #[test]
    fn declarations_load_as_data() {
        common::ensure_installed();
        let base = &scripts().base;
        assert_eq!(base.targets.len(), 1);
        assert_eq!(base.modes.len(), 1);
        assert!(base.static_flags.is_some());
        assert_eq!(base.activations.len(), 1);
        assert_eq!(base.activations[0].id, "ping");
        assert_eq!(base.triggers.len(), 1);
        assert_eq!(base.triggers[0].id, "t1");
        assert_eq!(base.hand_triggers.len(), 1);
        assert_eq!(base.replacements.len(), 1);
        assert_eq!(base.replacements[0].id, "r1");
        assert_eq!(base.target_checks.len(), 1);
        assert!(base.target_checks.contains_key("cheap"));
        assert_eq!(base.resume.len(), 1);
        assert_eq!(base.grants.len(), 1);
    }

    #[test]
    fn trigger_when_and_run_fire() {
        common::ensure_installed();
        let when = adapt_trigger_when(site("triggers"), code(), "triggers", 0);
        let run = adapt_trigger_run(site("triggers"), code(), "triggers", 0);
        let state = scenario(json!({ "seed": "lx-trig", "p1": { "hand": ["lx-filler-00"] } }))
            .state()
            .clone();
        let event = jackioh_engine::wire::GameEvent::TurnStarted {
            player: PlayerId::P1,
            turn: 1,
            extra: None,
        };
        let mut state = state;
        let mut events = Vec::new();
        let mut rng = jackioh_engine::Rng::new(&state.seed, state.rng_cursor);
        let mut ctx = EffectContext::new(
            jackioh_engine::EngineSink::new(&mut state, &mut events, &mut rng),
            PlayerId::P1,
        );
        assert!(when(&mut ctx, &event));
        let kinds = run(&mut ctx, &event)
            .iter()
            .map(|effect| effect.kind)
            .collect::<Vec<_>>();
        assert_eq!(kinds, ["draw"]);
    }

    #[test]
    fn target_check_activation_and_sub_hooks_fire() {
        common::ensure_installed();
        let check = adapt_target_check(site("targetChecks"), code(), "cheap");
        let run = adapt_activation_run(site("activations"), code(), 0);
        let can = adapt_activation_can_activate(site("activations"), code(), 0);
        let state = scenario(json!({ "seed": "lx-trig", "p1": { "hand": ["lx-filler-00"] } }))
            .state()
            .clone();
        let mut state = state;
        let mut events = Vec::new();
        let mut rng = jackioh_engine::Rng::new(&state.seed, state.rng_cursor);
        {
            let card =
                jackioh_engine::query::zone_cards(&state, PlayerId::P1, jackioh_engine::OffFieldZone::Hand)
                    .into_iter()
                    .next()
                    .expect("a hand card");
            let selection = jackioh_engine::wire::Selection::None;
            let hook_args = jackioh_engine::script::TargetCheckArgs {
                state: &state,
                self_: &card,
                player: PlayerId::P1,
                radiant: false,
                candidate: None,
                selection: &selection,
            };
            assert!(check(hook_args));
            let condition = jackioh_engine::script::ConditionContext {
                state: &state,
                self_: &card,
                controller: PlayerId::P1,
                radiant: false,
                your_turn: true,
                zone: jackioh_engine::script::ConditionZone::Hand,
            };
            assert!(can(condition));
        }
        let mut ctx = EffectContext::new(
            jackioh_engine::EngineSink::new(&mut state, &mut events, &mut rng),
            PlayerId::P1,
        );
        let kinds = run(&mut ctx).iter().map(|effect| effect.kind).collect::<Vec<_>>();
        assert_eq!(kinds, ["damage"]);
        let resume = adapt_sub_hook(site("resume"), code(), "resume", "step_two");
        let kinds = resume(&mut ctx)
            .iter()
            .map(|effect| effect.kind)
            .collect::<Vec<_>>();
        assert_eq!(kinds, ["draw"]);
        let grants = adapt_sub_hook(site("grants"), code(), "grants", "extra");
        let kinds = grants(&mut ctx)
            .iter()
            .map(|effect| effect.kind)
            .collect::<Vec<_>>();
        assert_eq!(kinds, ["gainMana"]);
    }
}

// --------------------------------------------------------------------------------
// Readers: the probe fixture remembers board facts; the test reads them back.
// --------------------------------------------------------------------------------

#[test]
fn readers_probe_values() {
    use jackioh_engine::PlayerId;
    common::ensure_installed();
    let site = jackioh_luau::Site {
        card: "lx-probe",
        face: jackioh_luau::Face::Base,
        hook: "cry",
    };
    let code = fixture!("readers_probe");
    let mut state = scenario(json!({ "seed": "lx-probe", "p1": { "hand": ["lx-filler-00"] } }))
        .state()
        .clone();
    let probe_card =
        jackioh_engine::query::zone_cards(&state, PlayerId::P1, jackioh_engine::OffFieldZone::Hand)
            .into_iter()
            .next()
            .expect("a hand card");
    let probe_id = probe_card.id.clone();
    let mut events = Vec::new();
    let mut rng = jackioh_engine::Rng::new(&state.seed, state.rng_cursor);
    let mut ctx = EffectContext::new(
        jackioh_engine::EngineSink::new(&mut state, &mut events, &mut rng),
        PlayerId::P1,
    );
    // `remember` writes the running card's memory, so the probe runs as one
    // of the state's own cards.
    ctx.self_ = Some(probe_card);
    for effect in run_hook(site, code, &mut ctx).expect("probe runs") {
        (effect.apply)(&mut ctx);
    }
    drop(ctx);
    let probe = jackioh_engine::state::find_instance(&state, &probe_id)
        .expect("the probe card")
        .memory
        .get("probe")
        .cloned()
        .expect("the probe value");
    let probe: Value = probe;
    assert_eq!(probe["controller"], json!("p1"));
    assert_eq!(probe["radiant"], json!(false));
    assert_eq!(probe["x"], json!(0));
    assert_eq!(probe["fib"], json!(55));
    assert!(probe["myHeroHealth"].as_u64().unwrap() > 0);
    assert!(probe["foeHeroHealth"].as_u64().unwrap() > 0);
    assert!(probe["myHand"].as_u64().unwrap() >= 1);
    assert_eq!(probe["missingTags"], json!(0));
    assert_eq!(probe["missingCost"], json!(0));
    assert_eq!(probe["summoned"], json!(0));
    assert_eq!(probe["chosen"], json!(0));
}

// --------------------------------------------------------------------------------
// Verb kinds: every bound verb builds its effect through one shared table.
// --------------------------------------------------------------------------------

/// (verb, minimal args) pairs: each must build through `effect_of` with the
/// verb's own kind. The parity scenarios above prove the behaviour; this table
/// proves every other verb group is wired.
fn verb_cases() -> Vec<(&'static str, Value)> {
    vec![
        ("damage", json!({ "to": { "of": "enemyHero" }, "amount": 1 })),
        ("damage_all", json!({ "amount": 1 })),
        ("forced_attack_random", json!({ "attacker": { "of": "self" } })),
        ("destroy", json!({ "target": { "of": "self" } })),
        ("destroy_all", json!({ "side": "enemy" })),
        ("sacrifice", json!({ "target": { "of": "self" } })),
        ("draw", json!({ "count": 1 })),
        ("draw_from_library", json!({ "count": 1 })),
        ("discard", json!({ "target": { "of": "self" } })),
        ("discard_random", json!({ "count": 1 })),
        ("discard_hand", json!({ "player": "self" })),
        ("shuffle_card_into", json!({ "instanceId": "none" })),
        ("buff", json!({ "target": { "of": "self" }, "attack": 1 })),
        ("buff_all_units", json!({ "attack": 1 })),
        ("degrade", json!({ "target": { "of": "self" } })),
        ("upgrade", json!({ "target": { "of": "self" } })),
        (
            "enchant",
            json!({ "target": { "of": "self" }, "enchantment": { "kind": "targetEnemies" } }),
        ),
        ("summon", json!({ "defId": "lx-fx-001" })),
        ("summon_this", json!({})),
        ("heal", json!({ "target": { "of": "selfHero" }, "amount": 1 })),
        ("damage_enemy_or_heal_friend", json!({ "amount": 1 })),
        ("gain_mana", json!({ "amount": 1 })),
        ("exile", json!({ "target": { "of": "self" } })),
        ("exile_all", json!({ "side": "enemy" })),
        ("exile_hand", json!({ "player": "self" })),
        ("bounce", json!({ "target": { "of": "self" } })),
        ("flicker", json!({ "target": { "of": "self" } })),
        ("switch_position_of", json!({ "target": { "of": "self" } })),
        ("swap", json!({})),
        (
            "transform",
            json!({ "target": { "of": "self" }, "defId": "lx-fx-001" }),
        ),
        ("remember", json!({ "key": "k", "value": 1 })),
        ("gain_brittle", json!({ "target": { "of": "self" }, "n": 1 })),
        ("grant_tag", json!({ "tag": "Human" })),
        ("plague", json!({ "amount": 1 })),
        (
            "grant_keyword",
            json!({ "target": { "of": "self" }, "keyword": { "kind": "Rush" } }),
        ),
        ("win_game", json!({ "player": "self" })),
        ("reveal", json!({ "target": { "of": "self" } })),
        (
            "craft_card",
            json!({ "recipe": { "cost": 0, "type": "Unit", "adjective": "Test", "noun": "Card", "attack": 1, "health": 1, "keywords": [], "echo": 0, "hats": [] } }),
        ),
        ("fuse_cards", json!({ "defIds": ["lx-fx-001"] })),
    ]
}

/// The snake_case verb as its camelCase `Effect.kind`.
fn camel(verb: &str) -> String {
    let mut out = String::new();
    for (index, part) in verb.split('_').enumerate() {
        if index == 0 {
            out.push_str(part);
        } else {
            let mut chars = part.chars();
            out.extend(chars.next().iter().flat_map(|first| first.to_uppercase()));
            out.push_str(chars.as_str());
        }
    }
    out
}

#[test]
fn every_listed_verb_builds_its_kind() {
    let site = Site {
        card: "lx-kind",
        face: Face::Base,
        hook: "cry",
    };
    for (verb, args) in verb_cases() {
        let call = EffectCall {
            verb: verb.to_string(),
            args,
        };
        // Closure-taking verbs resolve their named face function from the card's
        // bytecode; this table covers data-only verbs, so a dummy suffices.
        let effect =
            effect_of(&call, site, b"verb-kinds").unwrap_or_else(|error| panic!("{verb} builds: {error}"));
        // One verb's kind is not its name in camelCase.
        let expected = if verb == "switch_position_of" {
            "switchPosition".to_string()
        } else {
            camel(verb)
        };
        assert_eq!(effect.kind, expected, "{verb} builds its own kind");
    }
}

// --------------------------------------------------------------------------------
// Sync: the committed module names every binding, and every binding is named.
// --------------------------------------------------------------------------------

/// The committed typed module, read at compile time: rule 4 bans the disk in this
/// crate, tests included.
fn module_text() -> &'static str {
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../cards/luau/jackioh.luau"))
}

#[test]
fn every_verb_has_a_typed_stub() {
    let module = module_text();
    for verb in all_verbs() {
        assert!(
            module.contains(&format!("function J.{verb}(")),
            "jackioh.luau stubs J.{verb}"
        );
    }
}

#[test]
fn every_ctx_reader_has_a_typed_stub() {
    let module = module_text();
    let source: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/reads.rs"));
    let mut missing = Vec::new();
    // Every context flavor's readers: the effect context and the predicate ones.
    let mut in_ctx: Option<&str> = None;
    for line in source.lines() {
        if line.starts_with("pub fn register_") {
            in_ctx = [
                "pub fn register_ctx(",
                "pub fn register_condition_ctx(",
                "pub fn register_hook_args(",
                "pub fn register_replacement_ctx(",
            ]
            .into_iter()
            .find(|prefix| line.starts_with(prefix));
            continue;
        }
        if line == "}" {
            in_ctx = None;
            continue;
        }
        if in_ctx.is_none() {
            continue;
        }
        if let Some(rest) = line.trim().strip_prefix("registry.add_method") {
            let name = rest
                .trim_start_matches("_mut")
                .trim_start_matches('(')
                .trim_start_matches('"')
                .split('"')
                .next()
                .unwrap_or("");
            if name.starts_with("subsystems_") || name.is_empty() {
                continue;
            }
            if !module.contains(&format!("\t{name}:")) {
                missing.push(name.to_string());
            }
        }
    }
    // Subsystem readers live under longer names; check a sample of those too.
    for name in ["subsystems_activation_paid", "subsystems_pareto_judgement_of"] {
        assert!(
            module.contains(&format!("\t{name}:")),
            "jackioh.luau stubs ctx:{name}"
        );
    }
    assert!(missing.is_empty(), "readers without a stub: {missing:?}");
}

// --------------------------------------------------------------------------------
// Hook adapters: every hook type over the fixture scripts.
// --------------------------------------------------------------------------------
