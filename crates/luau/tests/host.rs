//! The Luau host's tests (#442 L5–L8), over the card-shaped modules in `fixtures/`: the sandbox, the
//! interrupt's cap, the integer boundary, `J.div` and `J.rem`, statelessness, nesting, threads, the
//! loader and the lint. Each fixture has a card id of its own, since a thread's VM caches one chunk
//! per id.

use jackioh_engine::config::DECK_SIZE;
use jackioh_engine::query::hero_of;
use jackioh_engine::testkit::{
    CardDef, CardDefs, EffectContext, EngineSink, IndexMap, PlayerId, Rng, catalog_override, json_as,
    register_catalog, register_scripts, scenario,
};
use jackioh_luau::config::LUAU_HOOK_INTERRUPTS;
use jackioh_luau::numbers::{div, number_to_i32, rem, to_json};
use jackioh_luau::sandbox::{ABSENT_LIBRARIES, REMOVED_GLOBALS, REMOVED_MATH, new_sandbox};
use jackioh_luau::vm::{call_hook, declared_hooks};
use jackioh_luau::{EffectCall, Face, Finding, LuauError, Rule, Site, compile, lint, load_card};
use mlua::{Table, Value};
use serde_json::json;

/// A source compiled (and linted) to bytecode that lives as long as the test run, as a card's
/// `include_bytes!` will.
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

fn site(card: &'static str, face: Face, hook: &'static str) -> Site {
    Site { card, face, hook }
}

/// A vanilla Unit `fx-<index>`, as the engine's rules fixtures write one.
fn unit_def(index: i32) -> CardDef {
    json_as(json!({
        "id": format!("fx-{index}"),
        "index": index.to_string(),
        "name": format!("Fixture Unit {index}"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "attack": 2, "health": 2, "keywords": [], "text": "vanilla" },
        "radiant": { "attack": 4, "health": 4, "keywords": [], "text": "vanilla" },
    }))
}

/// `f` with a context over a scenario at rest, seat 1 in control. The thread's catalog is
/// `DECK_SIZE` vanilla units, which `scenario` needs for its filler decks.
fn with_ctx<R>(f: impl FnOnce(&mut EffectContext<'_>) -> R) -> R {
    if catalog_override().is_none() {
        let defs: CardDefs = (1..=DECK_SIZE)
            .map(|index| {
                let def = unit_def(index);
                (def.id.clone(), def)
            })
            .collect();
        register_catalog(defs);
    }
    let mut state = scenario(json!({})).state().clone();
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut ctx = EffectContext::new(EngineSink::new(&mut state, &mut events, &mut rng), PlayerId::P1);
    f(&mut ctx)
}

/// The call `J.damage({ to = { of = "enemyHero" }, amount = amount })` makes.
fn hit(amount: i32) -> EffectCall {
    EffectCall {
        verb: "damage".into(),
        args: json!({ "amount": amount, "to": { "of": "enemyHero" } }),
    }
}

fn error_text<T>(result: Result<T, LuauError>) -> String {
    match result {
        Ok(_) => panic!("expected an error"),
        Err(error) => error.to_string(),
    }
}

// ---- the sandbox (L5) ----------------------------------------------------------------------------

#[test]
fn every_removed_global_is_nil() {
    let sandbox = new_sandbox().unwrap();
    let absent = REMOVED_GLOBALS
        .iter()
        .chain(&ABSENT_LIBRARIES)
        .chain(&["buffer", "vector", "integer"]);
    for name in absent {
        assert!(sandbox.env.raw_get::<Value>(*name).unwrap().is_nil(), "{name}");
    }
    let math: Table = sandbox.env.raw_get("math").unwrap();
    for name in REMOVED_MATH {
        assert!(math.raw_get::<Value>(name).unwrap().is_nil(), "math.{name}");
    }
    for library in ["string", "table", "math", "bit32", "utf8"] {
        assert!(
            sandbox.env.raw_get::<Value>(library).unwrap().is_table(),
            "{library}"
        );
    }
}

#[test]
fn writing_a_global_fails() {
    let code = fixture!("write_global");
    let text = error_text(with_ctx(|ctx| {
        call_hook(site("fixture-write-global", Face::Base, "cry"), code, ctx)
    }));
    assert!(text.starts_with("fixture-write-global base cry: "), "{text}");
    assert!(text.contains("readonly"), "{text}");
}

#[test]
fn require_answers_only_the_jackioh_module() {
    let error = load_card("fixture-require-other", fixture!("require_other")).err();
    let Some(LuauError::Load { card, message }) = error else {
        panic!("expected a load error");
    };
    assert_eq!(card, "fixture-require-other");
    assert!(
        message.contains(r#"require answers only "@jackioh", not "other""#),
        "{message}"
    );
}

#[test]
fn a_loop_that_never_ends_stops_at_the_cap_naming_the_card() {
    let code = fixture!("endless");
    let text = error_text(with_ctx(|ctx| {
        call_hook(site("fixture-endless", Face::Base, "cry"), code, ctx)
    }));
    let expected = format!("fixture-endless base cry: stopped after {LUAU_HOOK_INTERRUPTS} interrupts");
    assert!(text.contains(&expected), "{text}");

    // A hook that catches the stop with `pcall` is still stopped: the budget stays spent, so the
    // next interrupt fails it again.
    let caught = bytecode(
        "endless_caught.luau",
        "return { base = { cry = function(ctx)\n\tpcall(function() while true do end end)\n\treturn nil\nend } }\n",
    );
    let text = error_text(with_ctx(|ctx| {
        call_hook(site("fixture-endless-caught", Face::Base, "cry"), caught, ctx)
    }));
    let expected =
        format!("fixture-endless-caught base cry: stopped after {LUAU_HOOK_INTERRUPTS} interrupts");
    assert!(text.contains(&expected), "{text}");

    let at_load = bytecode("endless_load.luau", "while true do\nend\nreturn {}\n");
    let text = error_text(load_card("fixture-endless-load", at_load));
    assert!(
        text.contains("fixture-endless-load base load: stopped after"),
        "{text}"
    );
}

// ---- integers at the boundary (L6) ---------------------------------------------------------------

#[test]
fn a_bad_number_in_an_effect_fails_naming_the_card_face_hook_and_value() {
    let code = fixture!("bad_numbers");
    let card = "fixture-bad-numbers";
    let cases = [
        (Face::Base, "cry", "1.5"),
        (Face::Base, "death", "NaN"),
        (Face::Base, "startOfTurn", "inf"),
        (Face::Radiant, "cry", "2147483648"),
    ];
    for (face, hook, value) in cases {
        let at = site(card, face, hook);
        let error = LuauError::Number {
            site: at,
            value: value.into(),
        };
        assert_eq!(with_ctx(|ctx| call_hook(at, code, ctx)), Err(error), "{at}");
    }
    let text = error_text(with_ctx(|ctx| {
        call_hook(site(card, Face::Base, "endOfTurn"), code, ctx)
    }));
    assert!(text.starts_with("fixture-bad-numbers base endOfTurn: "), "{text}");
    assert!(
        text.contains("3.5 is not an integer in i32's range (L6)"),
        "{text}"
    );
}

#[test]
#[should_panic(expected = "fixture-bad-numbers base cry: 1.5 is not an integer")]
fn a_loaded_hook_panics_with_the_error() {
    let Ok(scripts) = load_card("fixture-bad-numbers", fixture!("bad_numbers")) else {
        panic!("the fixture loads");
    };
    let cry = scripts.base.cry.expect("the base face declares cry");
    with_ctx(|ctx| cry(ctx));
}

#[test]
fn the_walk_takes_only_integers_in_i32() {
    assert_eq!(number_to_i32(&Value::Number(2.0)), Ok(2));
    assert_eq!(number_to_i32(&Value::Number(-0.0)), Ok(0));
    assert_eq!(number_to_i32(&Value::Integer(i32::MIN.into())), Ok(i32::MIN));
    let too_large = mlua::Integer::from(i32::MAX) + 1;
    assert_eq!(
        number_to_i32(&Value::Integer(too_large)),
        Err("2147483648".into())
    );
    assert_eq!(number_to_i32(&Value::Number(-0.5)), Err("-0.5".into()));
    assert_eq!(number_to_i32(&Value::Number(f64::NAN)), Err("NaN".into()));
    assert_eq!(
        number_to_i32(&Value::Number(f64::NEG_INFINITY)),
        Err("-inf".into())
    );
    assert_eq!(number_to_i32(&Value::Number(3e10)), Err("30000000000".into()));
}

#[test]
fn the_walk_turns_lists_and_records_into_json() {
    let sandbox = new_sandbox().unwrap();
    let eval = |source: &str| -> Value {
        sandbox
            .lua
            .load(source)
            .set_environment(sandbox.env.clone())
            .eval()
            .unwrap()
    };
    let at = site("fixture-walk", Face::Base, "cry");
    let walked = to_json(&eval("return { 1, { b = true, a = 'x' }, {} }"), at).unwrap();
    assert_eq!(
        serde_json::to_string(&walked).unwrap(),
        r#"[1,{"a":"x","b":true},{}]"#
    );

    let refused = [
        ("return { 1, a = 2 }", "neither 1..n nor all strings"),
        ("return { 1, nil, 3 }", "neither 1..n nor all strings"),
        (
            "return { f = function() end }",
            "a value of type function cannot cross into Rust (L6)",
        ),
        ("local t = {}\nt.t = t\nreturn t", "nested deeper than 32 tables"),
    ];
    for (source, reason) in refused {
        match to_json(&eval(source), at) {
            Err(LuauError::Hook { site, message }) => {
                assert_eq!(site, at);
                assert!(message.contains(reason), "{source}: {message}");
            }
            other => panic!("{source}: {other:?}"),
        }
    }
}

#[test]
fn j_div_and_j_rem_truncate_as_rust_does() {
    let code = fixture!("div_rem");
    let rust = [-7 / 2, -7 % 2, 7 / -2, 7 % -2, -7 / -2, -7 % -2];
    assert_eq!(rust, [-3, -1, -3, 1, 3, -1]);
    let calls = with_ctx(|ctx| call_hook(site("fixture-div-rem", Face::Base, "cry"), code, ctx));
    assert_eq!(calls, Ok(rust.map(hit).to_vec()));

    let text = error_text(with_ctx(|ctx| {
        call_hook(site("fixture-div-rem", Face::Base, "death"), code, ctx)
    }));
    assert!(text.contains("J.div(1, 0) has no i32 answer"), "{text}");
    assert_eq!(div(i32::MIN, -1), None);
    assert_eq!(rem(i32::MIN, -1), None);
}

// ---- one VM per thread (L8) ----------------------------------------------------------------------

#[test]
fn a_module_level_counter_reads_the_same_on_every_call() {
    let code = fixture!("counter");
    let at = site("fixture-counter", Face::Base, "cry");
    for _ in 0..3 {
        assert_eq!(with_ctx(|ctx| call_hook(at, code, ctx)), Ok(vec![hit(1)]));
    }
}

#[test]
fn a_hook_runs_another_scripts_hook_from_a_reader() {
    let Ok(inner) = load_card("fixture-nest-inner", fixture!("nest_inner")) else {
        panic!("the inner fixture loads");
    };
    register_scripts(IndexMap::from([("fixture-nest-inner".to_string(), inner)]));
    let code = fixture!("nest_outer");
    let calls = with_ctx(|ctx| call_hook(site("fixture-nest-outer", Face::Base, "cry"), code, ctx));
    assert_eq!(calls, Ok(vec![hit(2)]));

    let Ok(outer) = load_card("fixture-nest-outer", code) else {
        panic!("the outer fixture loads");
    };
    let cry = outer.base.cry.expect("the base face declares cry");
    let kinds: Vec<&str> = with_ctx(|ctx| cry(ctx).iter().map(|effect| effect.kind).collect());
    assert_eq!(kinds, ["damage"]);
}

/// `board`'s base cry as calls, and its radiant cry, loaded, as effect kinds.
fn board() -> (Vec<EffectCall>, Vec<&'static str>, i32) {
    let code = fixture!("board");
    let Ok(scripts) = load_card("fixture-board", code) else {
        panic!("the board fixture loads");
    };
    let cry = scripts.radiant.cry.expect("the radiant face declares cry");
    with_ctx(|ctx| {
        let calls = call_hook(site("fixture-board", Face::Base, "cry"), code, ctx).unwrap();
        let kinds = cry(ctx).iter().map(|effect| effect.kind).collect();
        (calls, kinds, hero_of(ctx.state, PlayerId::P1).health)
    })
}

#[test]
fn the_same_effects_come_out_on_two_threads() {
    let here = board();
    let destroy_chosen = EffectCall {
        verb: "destroy".into(),
        args: json!({ "target": { "of": "chosen" } }),
    };
    assert_eq!(here.0, [hit(here.2 / 7), destroy_chosen]);
    assert_eq!(here.1, ["damage", "destroy"]);
    let there = std::thread::scope(|scope| {
        let first = scope.spawn(board);
        let second = scope.spawn(board);
        [first.join().unwrap(), second.join().unwrap()]
    });
    assert_eq!(there, [here.clone(), here]);
}

// ---- the loader ----------------------------------------------------------------------------------

#[test]
fn load_card_reads_the_declared_hooks() {
    let code = fixture!("bad_numbers");
    let declared = declared_hooks("fixture-bad-numbers", code);
    let expected = vec![
        (Face::Base, "cry"),
        (Face::Base, "death"),
        (Face::Base, "startOfTurn"),
        (Face::Base, "endOfTurn"),
        (Face::Radiant, "cry"),
    ];
    assert_eq!(declared, Ok(expected));

    let Ok(scripts) = load_card("fixture-bad-numbers", code) else {
        panic!("the fixture loads");
    };
    let (base, radiant) = (&scripts.base, &scripts.radiant);
    assert!(base.cry.is_some() && base.death.is_some());
    assert!(base.start_of_turn.is_some() && base.end_of_turn.is_some());
    assert!(base.start_of_game.is_none() && base.delayed.is_none() && base.on_play_hook.is_none());
    assert!(base.after_attack.is_none() && base.start_of_opponent_turn.is_none());
    assert!(radiant.cry.is_some() && radiant.death.is_none() && radiant.end_of_turn.is_none());

    let typo = bytecode("typo.luau", "return { base = { cri = function() end } }\n");
    assert_eq!(
        error_text(load_card("fixture-typo", typo)),
        r#"fixture-typo: base declares "cri", which is not a hook"#
    );
    let stray = bytecode("stray.luau", "return { bsae = {} }\n");
    assert_eq!(
        error_text(load_card("fixture-stray", stray)),
        r#"fixture-stray: the module declares "bsae", which is not base or radiant"#
    );
    let not_a_function = bytecode("value.luau", "return { radiant = { cry = 1 } }\n");
    assert_eq!(
        error_text(load_card("fixture-value", not_a_function)),
        "fixture-value: radiant cry is of type integer, not a function"
    );
}

// ---- compiling and the lint (L6, L7) -------------------------------------------------------------

#[test]
fn the_lint_refuses_each_rules_example() {
    let header = "local n, t, list = 1, {}, {}\n";
    let cases = [
        ("next(t)", Rule::Next),
        ("local f = pairs", Rule::Pairs),
        ("local holder = { next = 1 }", Rule::Next),
        ("table.sort(list)", Rule::TableSort),
        ("for _, x in list do end", Rule::GenericFor),
        ("for k, v in t:entries() do end", Rule::GenericFor),
        ("for _, x in ipairs(list), n do end", Rule::GenericFor),
        ("n / 2", Rule::Slash),
        ("n /= 2", Rule::Slash),
        ("`{n / 2}`", Rule::Slash),
        ("n ^ 2", Rule::Caret),
        ("n ^= 2", Rule::Caret),
        ("math.random(1, 6)", Rule::Math("random".into())),
        ("math.sqrt(n)", Rule::Math("sqrt".into())),
        ("local h = math.huge", Rule::Math("huge".into())),
        ("local m = math", Rule::Math(String::new())),
        ("print(n)", Rule::RemovedGlobal("print".into())),
        ("os.time()", Rule::RemovedGlobal("os".into())),
        ("local d = debug", Rule::RemovedGlobal("debug".into())),
    ];
    assert_eq!(lint("example.luau", header), vec![]);
    for (example, rule) in cases {
        let source = format!("{header}{example}\n");
        let finding = Finding {
            file: "example.luau".into(),
            line: 2,
            rule,
        };
        assert_eq!(lint("example.luau", &source), vec![finding], "{example}");
    }
    let rules: Vec<Rule> = lint("pairs.luau", "for k, v in pairs(t) do end\n")
        .into_iter()
        .map(|finding| finding.rule)
        .collect();
    assert_eq!(rules, [Rule::GenericFor, Rule::Pairs]);
}

#[test]
fn a_finding_names_the_file_line_and_rule() {
    assert_eq!(
        error_text(compile("dirty.luau", "local a = 1\n\nlocal b = a / 2\n")),
        "dirty.luau:3: `/` is refused: it makes fractions (L6); use `J.div`"
    );
}

#[test]
fn the_lint_accepts_the_clean_fixture() {
    assert_eq!(lint("clean.luau", include_str!("fixtures/clean.luau")), vec![]);
    let code = fixture!("clean");
    let calls = with_ctx(|ctx| call_hook(site("fixture-clean", Face::Base, "cry"), code, ctx));
    assert_eq!(calls, Ok(vec![hit(7)]));
}

#[test]
fn a_syntax_error_names_the_file() {
    let error = compile("broken.luau", "local x = ");
    let Err(LuauError::Syntax { file, message }) = &error else {
        panic!("expected a syntax error, got {error:?}");
    };
    assert_eq!(file, "broken.luau");
    assert!(message.starts_with("1: "), "{message}");
    assert!(
        error_text(error.clone()).starts_with("broken.luau:1: "),
        "{message}"
    );
}
