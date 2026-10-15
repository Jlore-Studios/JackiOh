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
                "pub fn register_target_check_args(",
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

// --------------------------------------------------------------------------------
// #464 part 28: one fixture test per API gap.
// --------------------------------------------------------------------------------

/// Runs a fixture's `cry` as `card` in a fresh scenario and returns the state with the
/// running card's memory.
fn run_cry(card: &'static str, fixture: &'static str, opts: Value) -> (Value, IndexMap<String, Value>) {
    use jackioh_engine::testkit::{EngineSink, PlayerId, Rng};
    common::ensure_installed();
    let site = Site {
        card,
        face: Face::Base,
        hook: "cry",
    };
    let code = match fixture {
        "gap_empty_tables" => fixture!("gap_empty_tables"),
        "gap_for_each_data" => fixture!("gap_for_each_data"),
        "gap_readers" => fixture!("gap_readers"),
        "gap_kill_credit" => fixture!("gap_kill_credit"),
        _ => panic!("no such gap fixture: {fixture}"),
    };
    let mut state = scenario(opts).state().clone();
    let probe = jackioh_engine::zone_cards(&state, PlayerId::P1, jackioh_engine::OffFieldZone::Hand)
        .into_iter()
        .next()
        .expect("a hand card to run as")
        .clone();
    let probe_id = probe.id.clone();
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut ctx = EffectContext::new(EngineSink::new(&mut state, &mut events, &mut rng), PlayerId::P1);
    ctx.self_ = Some(probe);
    for effect in run_hook(site, code, &mut ctx).expect("the cry runs") {
        (effect.apply)(&mut ctx);
    }
    drop(ctx);
    let memory = jackioh_engine::state::find_instance(&state, &probe_id)
        .expect("the probe card")
        .memory
        .clone();
    (serde_json::to_value(&state).expect("state serializes"), memory)
}

/// Gap 1: a `cards` function answering `{}` names no cards instead of panicking, and instances
/// with empty lists (`grantedKeywords`) round-trip through `numbering_order` — including a
/// literal `{}`.
#[test]
fn gap1_empty_tables_round_trip() {
    // The fixture's sub-hooks are allowlisted names, so the shape loads as a card does.
    load_card("lx-gap1-load", fixture!("gap_empty_tables")).expect("gap1 loads");
    let (_state, memory) = run_cry(
        "lx-gap1",
        "gap_empty_tables",
        json!({ "seed": "lx-gap1", "p1": { "hand": ["lx-p-for-each"] } }),
    );
    let probe = memory.get("gap1").expect("the cry remembered gap1");
    assert!(
        probe
            .get("handSize")
            .and_then(|size| size.as_i64())
            .expect("handSize")
            >= 1,
        "a hand card ran"
    );
    assert_eq!(
        probe.get("orderedSize"),
        probe.get("handSize"),
        "every hand card round-trips"
    );
    assert_eq!(
        probe.get("emptySize"),
        Some(&json!(0)),
        "a literal empty table reads back as a list"
    );
    assert!(memory.get("gap1_each").is_none(), "the each hook never ran");
}

/// Gap 2: `for_each_card`'s `data` reaches its named `cards` and `each` functions. The `each`
/// hook deals `data.amount` (2) to both enemy units: the 2/2 stands at 0, the 3/3 at 1.
#[test]
fn gap2_for_each_card_data() {
    use jackioh_engine::testkit::PlayerId;
    common::ensure_installed();
    load_card("lx-gap2-load", fixture!("gap_for_each_data")).expect("gap2 loads");
    let site = Site {
        card: "lx-gap2",
        face: Face::Base,
        hook: "cry",
    };
    let code = fixture!("gap_for_each_data");
    let mut state = scenario(json!({
        "seed": "lx-gap2",
        "p1": { "hand": ["lx-p-for-each"] },
        "p2": { "field": ["lx-fx-001", "lx-fx-002"] },
    }))
    .state()
    .clone();
    let ids: Vec<String> = jackioh_engine::active_units_of(&state, PlayerId::P2)
        .iter()
        .map(|unit| unit.id.clone())
        .collect();
    assert_eq!(ids.len(), 2, "two enemy units to hit");
    // Same runner as `run_cry`, inlined for the state it keeps.
    {
        use jackioh_engine::testkit::{EngineSink, Rng};
        let probe = jackioh_engine::zone_cards(&state, PlayerId::P1, jackioh_engine::OffFieldZone::Hand)
            .into_iter()
            .next()
            .expect("a hand card to run as")
            .clone();
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut ctx = EffectContext::new(EngineSink::new(&mut state, &mut events, &mut rng), PlayerId::P1);
        ctx.self_ = Some(probe);
        for effect in run_hook(site, code, &mut ctx).expect("the cry runs") {
            (effect.apply)(&mut ctx);
        }
    }
    // Damage needs no state check to land: the 2/2 stands at 0, the 3/3 at 1, so each hook
    // dealt exactly `data.amount` (2).
    let healths: Vec<i32> = jackioh_engine::active_units_of(&state, PlayerId::P2)
        .iter()
        .map(|unit| jackioh_engine::unit_view(&state, unit).health)
        .collect();
    assert_eq!(healths.len(), 2, "both enemy units were hit");
    assert!(healths.contains(&0), "the 2/2 took exactly data.amount");
    assert!(healths.contains(&1), "the 3/3 took exactly data.amount");
}

/// Gaps 3 and 4: the condition, the preview and the cost hook read the turn, the earlier plays,
/// the counters and the board. The turn is pinned to 7 with p1 active, so the hooks cannot pass
/// on defaults.
#[test]
fn gap3_shared_readers_on_condition_preview_and_cost() {
    use jackioh_engine::script::{ConditionContext, ConditionZone, CostArgs};
    use jackioh_engine::testkit::PlayerId;
    use jackioh_engine::zones::open_zones;
    use jackioh_engine::{Row, query::fusable_permanents_of};
    common::ensure_installed();
    let scripts = load_card("lx-gap3", fixture!("gap_shared_readers")).expect("gap3 loads");
    let mut state = scenario(json!({ "seed": "lx-gap3", "p1": { "hand": ["lx-p-for-each"] } }))
        .state()
        .clone();
    state.turn = 7;
    state.active = PlayerId::P1;
    let me = jackioh_engine::zone_cards(&state, PlayerId::P1, jackioh_engine::OffFieldZone::Hand)
        .into_iter()
        .next()
        .expect("a hand card to read on");
    let condition = scripts.base.condition_met.expect("conditionMet");
    assert!(
        condition(ConditionContext {
            state: &state,
            self_: &me,
            controller: PlayerId::P1,
            radiant: false,
            zone: ConditionZone::Hand,
            your_turn: true,
        }),
        "turn, played_earlier and params_view read on the condition context"
    );
    let preview = scripts.base.preview.expect("preview");
    let values = preview(ConditionContext {
        state: &state,
        self_: &me,
        controller: PlayerId::P1,
        radiant: false,
        zone: ConditionZone::Hand,
        your_turn: true,
    });
    assert_eq!(values.len(), 1);
    let expected = open_zones(&state, PlayerId::P1, Row::Units).len()
        + fusable_permanents_of(&state, PlayerId::P1, None).len();
    assert_eq!(
        values[0].value, expected as i32,
        "open zones plus fusable permanents"
    );
    let cost = scripts.base.cost.expect("cost");
    assert_eq!(
        cost(CostArgs {
            state: &state,
            instance: &me
        }),
        7,
        "the cost hook reads the turn number"
    );
}

/// Gaps 5, 6, 7, 9 and 12: `given_lucky_on`, the events window, `J.config`, `chancePercent`,
/// `left_field_since_resolved` and `nil` for absent answers.
#[test]
fn gap5_6_7_9_12_readers_events_config_chance_and_nil() {
    let (state, memory) = run_cry(
        "lx-gap579",
        "gap_readers",
        json!({ "seed": "lx-gap579", "p1": { "hand": ["lx-p-for-each"] } }),
    );
    let probe = memory.get("gap579").expect("the cry remembered gap579");
    let state_turn: i64 = state.get("turn").and_then(|turn| turn.as_i64()).expect("turn");
    assert_eq!(
        probe.get("turnNumber"),
        Some(&json!(state_turn)),
        "turn() answers the turn number"
    );
    assert_eq!(
        probe.get("activeIsMe"),
        Some(&json!(true)),
        "turn() answers the active player"
    );
    assert_eq!(
        probe.get("lucky"),
        Some(&json!(0)),
        "no granted Lucky on the probe"
    );
    assert!(
        probe.get("eventsFrom").and_then(|from| from.as_u64()).is_some(),
        "events_from() is a number"
    );
    assert_eq!(
        probe.get("configHealth"),
        Some(&json!(jackioh_engine::config::HERO_HEALTH)),
        "J.config reads the engine's constant"
    );
    assert_eq!(
        probe.get("configChain"),
        Some(&json!(jackioh_engine::config::CHAIN_OF_THOUGHT_REPEATS)),
        "J.config reads the engine's constant"
    );
    assert_eq!(
        probe.get("missingIsNil"),
        Some(&json!(true)),
        "an absent answer is nil, not truthy null"
    );
    assert_eq!(probe.get("missingType"), Some(&json!("nil")), "type(nil) is nil");
    assert_eq!(
        probe.get("leftField"),
        Some(&json!(false)),
        "a cardPlayed is not a departure since a resolution"
    );
}

/// Gap 9: the self-snapshot `killer_of`. The running card died carrying `last_damaged_by`, while
/// the live instance under its id carries none: `killer_of_self` still names the killer.
#[test]
fn gap9_killer_of_self_reads_the_snapshot() {
    use jackioh_engine::testkit::{EngineSink, PlayerId, Rng};
    common::ensure_installed();
    let site = Site {
        card: "lx-gap9",
        face: Face::Base,
        hook: "death",
    };
    let code = fixture!("gap_readers");
    let mut state = scenario(json!({
        "seed": "lx-gap9",
        "p1": { "field": ["lx-fx-001"] },
        "p2": { "field": ["lx-fx-002"] },
    }))
    .state()
    .clone();
    let victim = jackioh_engine::active_units_of(&state, PlayerId::P1)
        .into_iter()
        .next()
        .expect("a victim");
    let killer = jackioh_engine::active_units_of(&state, PlayerId::P2)
        .into_iter()
        .next()
        .expect("a killer");
    let victim_id = victim.id.clone();
    let mut snapshot = victim.clone();
    snapshot.last_damaged_by = Some(killer.id.clone());
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut ctx = EffectContext::new(EngineSink::new(&mut state, &mut events, &mut rng), PlayerId::P1);
    ctx.self_ = Some(snapshot);
    ctx.data
        .insert(jackioh_engine::prompts::SELF_KEY.to_string(), json!(true));
    for effect in run_hook(site, code, &mut ctx).expect("the death hook runs") {
        (effect.apply)(&mut ctx);
    }
    drop(ctx);
    let remembered = jackioh_engine::state::find_instance(&state, &victim_id)
        .expect("the victim")
        .memory
        .get("gap9_killer")
        .expect("the death hook remembered gap9_killer");
    assert_eq!(
        remembered.get("killerIsNil"),
        Some(&json!(false)),
        "the snapshot's killer is named"
    );
}

/// Gap 8: the generic aura, as data and as a named predicate. Both entries build with the
/// declared mods, and each `applies` keeps its own side out.
#[test]
fn gap8_generic_aura_entries() {
    use jackioh_engine::script::HookArgs;
    use jackioh_engine::testkit::PlayerId;
    common::ensure_installed();
    let scripts = load_card("lx-gap8", fixture!("gap_aura")).expect("gap8 loads");
    let state = scenario(json!({
        "seed": "lx-gap8",
        "p1": { "field": ["lx-fx-001"] },
        "p2": { "field": ["lx-fx-002"] },
    }))
    .state()
    .clone();
    let me = jackioh_engine::active_units_of(&state, PlayerId::P1)
        .into_iter()
        .next()
        .expect("an aura source");
    let foe = jackioh_engine::active_units_of(&state, PlayerId::P2)
        .into_iter()
        .next()
        .expect("an enemy unit");
    let aura = scripts.base.aura.expect("aura");
    let entries = aura(HookArgs {
        state: &state,
        self_: me,
        radiant: false,
    });
    assert_eq!(entries.len(), 2, "both aura entries build");
    assert_eq!(entries[0].mod_.attack, Some(2));
    assert_eq!(entries[0].mod_.max_health, Some(2));
    assert!(
        (entries[0].applies)(me),
        "the data filter keeps the source's side"
    );
    assert!(!(entries[0].applies)(foe), "the data filter turns the enemy away");
    assert_eq!(entries[1].mod_.max_health, Some(1));
    assert!((entries[1].applies)(me), "the predicate keeps the source");
    assert!(!(entries[1].applies)(foe), "the predicate turns lx-fx-002 away");
}

/// Gap 9: `with_kill_credit`'s `pairs` and `then` resolve by name and run with data. The cry
/// builds (name resolution fails loudly), and the named functions answer credits and effects
/// when called as the engine calls them.
#[test]
fn gap9_kill_credit_pairs_and_then_by_name() {
    use jackioh_engine::testkit::{EngineSink, PlayerId, Rng};
    use jackioh_luau::hooks::{call_named_kill_pairs, call_named_kill_then};
    common::ensure_installed();
    let code = fixture!("gap_kill_credit");
    load_card("lx-gap9kc-load", code).expect("gap9kc loads");
    let site = Site {
        card: "lx-gap9kc",
        face: Face::Base,
        hook: "cry",
    };
    let mut state = scenario(json!({
        "seed": "lx-gap9kc",
        "p1": { "hand": ["lx-p-for-each"] },
        "p2": { "field": ["lx-fx-001", "lx-fx-002"] },
    }))
    .state()
    .clone();
    let foes: Vec<String> = jackioh_engine::active_units_of(&state, PlayerId::P2)
        .iter()
        .map(|unit| unit.id.clone())
        .collect();
    assert_eq!(foes.len(), 2);
    let probe = jackioh_engine::zone_cards(&state, PlayerId::P1, jackioh_engine::OffFieldZone::Hand)
        .into_iter()
        .next()
        .expect("a hand card to run as")
        .clone();
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut ctx = EffectContext::new(EngineSink::new(&mut state, &mut events, &mut rng), PlayerId::P1);
    ctx.self_ = Some(probe);
    // Building the cry resolves both names; a misspelling panics here naming card, face and hook.
    let effects = run_hook(site, code, &mut ctx).expect("the cry builds");
    assert_eq!(effects.len(), 1, "one with_kill_credit");
    drop(ctx);
    // The pairs function answers one credit per enemy unit, victim and target both.
    let mut state2 = state.clone();
    let mut events2 = Vec::new();
    let mut rng2 = Rng::new(&state2.seed, state2.rng_cursor);
    let mut ctx2 = EffectContext::new(
        EngineSink::new(&mut state2, &mut events2, &mut rng2),
        PlayerId::P1,
    );
    let credits = call_named_kill_pairs(site, code, "kill_pairs", &mut ctx2, &json!({})).expect("pairs run");
    assert_eq!(credits.len(), 2, "one pair per enemy unit");
    for credit in &credits {
        assert!(foes.contains(&credit.victim_id), "the victim is a foe");
        assert_eq!(credit.to_id, credit.victim_id, "each pair names its own victim");
    }
    drop(ctx2);
    // The then function answers one ping from the pair and the data it was built with.
    let pair = credits.into_iter().next().expect("a pair");
    let effects =
        call_named_kill_then(site, code, "kill_then", &pair, &json!({ "ping": 1 })).expect("then runs");
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].kind, "damage");
}

/// Gap 10: one function under two hooks shares one adapter, so `Arc::ptr_eq` holds the way it
/// does for a Rust script that clones one hook (`classicplus-070`).
#[test]
fn gap10_one_function_under_two_hooks_shares_its_adapter() {
    common::ensure_installed();
    let scripts = load_card("lx-gap10", fixture!("gap_shared_hook")).expect("gap10 loads");
    let start = scripts.base.start_of_turn.expect("startOfTurn");
    let end = scripts.base.end_of_turn.expect("endOfTurn");
    assert!(Arc::ptr_eq(&start, &end), "one function, one adapter");
    let radiant_start = scripts.radiant.start_of_turn.expect("radiant startOfTurn");
    assert!(
        Arc::ptr_eq(&start, &radiant_start),
        "radiant = base shares it too"
    );
}

/// Gap 14: one function held by both faces' hooks — separate face tables, whose params
/// differ — shares one adapter, so `Arc::ptr_eq` holds between the base and Radiant faces'
/// hooks (`classic-082`, `classicplus-047`).
#[test]
fn gap14_one_function_across_both_faces_shares_its_adapter() {
    common::ensure_installed();
    let scripts = load_card("lx-gap14", fixture!("gap_shared_faces")).expect("gap14 loads");
    let base = scripts.base.death.expect("base death");
    let radiant = scripts.radiant.death.expect("radiant death");
    assert!(
        Arc::ptr_eq(&base, &radiant),
        "one function, one adapter across faces"
    );
}

/// Gap 14, running-face resolution: the shared `cry`'s named `each_card` resolves on the face
/// whose run it is — a base run remembers `base:…`, a radiant run `radiant:…`
/// (`classicplus-012-7`'s shape: one `cry`, per-face closers).
#[test]
fn gap14_shared_hook_resolves_named_hooks_on_the_running_face() {
    use jackioh_engine::testkit::{EngineSink, PlayerId, Rng};
    common::ensure_installed();
    let scripts = load_card("lx-gap14b", fixture!("gap_shared_faces")).expect("gap14 loads");
    // The faces share the one `cry` adapter.
    assert!(
        Arc::ptr_eq(
            scripts.base.cry.as_ref().expect("base cry"),
            scripts.radiant.cry.as_ref().expect("radiant cry")
        ),
        "one cry, one adapter across faces"
    );
    for (radiant, want) in [(false, "base:lx-target"), (true, "radiant:lx-target")] {
        let mut state = scenario(json!({ "seed": "lx-gap14", "p1": { "hand": ["lx-p-for-each"] } }))
            .state()
            .clone();
        let probe = jackioh_engine::zone_cards(&state, PlayerId::P1, jackioh_engine::OffFieldZone::Hand)
            .into_iter()
            .next()
            .expect("a hand card to run as")
            .clone();
        let probe_id = probe.id.clone();
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut ctx = EffectContext::new(EngineSink::new(&mut state, &mut events, &mut rng), PlayerId::P1);
        ctx.self_ = Some(probe);
        ctx.radiant = radiant;
        // The adapter, not `run_hook`: only the adapter resolves named hooks.
        let cry = scripts.base.cry.as_ref().expect("base cry");
        for effect in cry(&mut ctx) {
            (effect.apply)(&mut ctx);
        }
        drop(ctx);
        let memory = jackioh_engine::state::find_instance(&state, &probe_id)
            .expect("the probe card")
            .memory
            .clone();
        assert_eq!(
            memory.get("gap14_face").and_then(|face| face.as_str()),
            Some(want),
            "a run with radiant = {radiant} fires that face's each_card"
        );
    }
}

/// One function under a different hook on each face — `cry` on the base face, only `death` on the
/// Radiant face (#871). An adapter looks its function up under the key it was built for, so the
/// faces get one adapter each, and each runs its own face's function, its named `each_card`
/// included, whatever face the context names (a Radiant ingredient fused into a base host, #765).
/// One adapter shared by the two looked `cry` up on the Radiant face and found nothing.
#[test]
fn one_function_under_another_hook_on_each_face_runs_on_both_faces() {
    use jackioh_engine::testkit::{EngineSink, PlayerId, Rng};
    common::ensure_installed();
    let scripts = load_card("lx-other-keys", fixture!("shared_other_keys")).expect("loads");
    assert!(scripts.base.death.is_none() && scripts.radiant.cry.is_none());
    let cry = scripts.base.cry.as_ref().expect("base cry");
    let death = scripts.radiant.death.as_ref().expect("radiant death");
    assert!(!Arc::ptr_eq(cry, death), "one adapter a face");
    for (hook, want) in [(cry, "base:lx-target"), (death, "radiant:lx-target")] {
        for radiant in [false, true] {
            let mut state = scenario(json!({ "seed": "lx-other-keys", "p1": { "hand": ["lx-p-for-each"] } }))
                .state()
                .clone();
            let probe = jackioh_engine::zone_cards(&state, PlayerId::P1, jackioh_engine::OffFieldZone::Hand)
                .into_iter()
                .next()
                .expect("a hand card to run as")
                .clone();
            let probe_id = probe.id.clone();
            let mut events = Vec::new();
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut ctx =
                EffectContext::new(EngineSink::new(&mut state, &mut events, &mut rng), PlayerId::P1);
            ctx.self_ = Some(probe);
            ctx.radiant = radiant;
            for effect in hook(&mut ctx) {
                (effect.apply)(&mut ctx);
            }
            drop(ctx);
            let memory = jackioh_engine::state::find_instance(&state, &probe_id)
                .expect("the probe card")
                .memory
                .clone();
            assert_eq!(
                memory.get("other_keys_face").and_then(|face| face.as_str()),
                Some(want),
                "{want}'s face runs its own function with radiant = {radiant}"
            );
        }
    }
}
