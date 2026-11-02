//! R2521003 (issue #2521): `runActivation` with `spendUse` (S #26 ZolarKeth's "Activate your
//! Universities" beside its "A University can be Activated at most once each turn") runs another card's
//! ability only when that ability has not been used this turn, counted as its own Activate counts it
//! (B3.2 rules 1, 3 and 9), and then spends every use it has this turn. The run is still no activation
//! of the card's (no `activated` event) and R1876000's refusal of its own use is still not read.
//! Without `spendUse` nothing changes (R1876001).
//!
//! Fixtures are this file's own: `fx-` defs indexed from 25220 (BUILD §0).

use jackioh_engine::effects::{ActivationOfArgs, gain_mana, refuse_own_activate, run_activation};
use jackioh_engine::subsystems::activate::why_cannot_activate_ability;
use jackioh_engine::testkit::PlayerId::P1;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::unit_def;
use crate::rules::fixtures::harness::{both, setup_catalog};

/// "Activate: Gain 1 mana" (a University's shape: once a turn, no target, no mode).
const SCHOOL: &str = "fx-25220";
/// "Activate 2: Gain 1 mana".
const ACADEMY: &str = "fx-25221";
/// "Activate: Activate your schools and academies", spending their uses (ZolarKeth's shape).
const DEAN: &str = "fx-25222";
const GAIN: i32 = 1;
const ACADEMY_USES: i32 = 2;
/// The refusal of a once-a-turn ability used this turn (`why_cannot_activate_ability`).
const SPENT: &str = "that ability has already been used this turn";

fn school() -> CardDef {
    unit_def(25220, json!({ "attack": 0, "health": 5 }))
}

fn academy() -> CardDef {
    unit_def(25221, json!({ "attack": 0, "health": 5 }))
}

fn dean() -> CardDef {
    unit_def(25222, json!({ "attack": 1, "health": 5 }))
}

fn ability(id: &str, uses: i32, run: Hook) -> ActivationDecl {
    ActivationDecl {
        id: id.to_string(),
        label: id.to_string(),
        uses: ActivationUses::Count(uses),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run,
    }
}

fn gain() -> Hook {
    hook(|_ctx| vec![gain_mana(json_as(json!({ "amount": GAIN })))])
}

fn run_of(id: &str, spend: bool) -> Effect {
    run_activation(ActivationOfArgs {
        instance_id: Some(id.to_string()),
        spend_use: spend.then_some(true),
        ..ActivationOfArgs::default()
    })
}

fn scripts_of(uses: i32) -> CardScripts {
    both(Script {
        activations: vec![ability("study", uses, gain())],
        ..Script::default()
    })
}

fn dean_script() -> CardScripts {
    both(Script {
        activations: vec![ability(
            "rouse",
            1,
            hook(|ctx| {
                let state: &GameState = &*ctx.state;
                jackioh_engine::zones::active_units_of(state, ctx.controller)
                    .into_iter()
                    .filter(|unit| unit.def_id == SCHOOL || unit.def_id == ACADEMY)
                    .map(|unit| run_of(&unit.id, true))
                    .collect()
            }),
        )],
        ..Script::default()
    })
}

fn register() {
    setup_catalog();
    let mut catalog = registered_catalog().clone();
    for def in [school(), academy(), dean()] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts = registered_scripts();
    scripts.insert(SCHOOL.to_string(), scripts_of(1));
    scripts.insert(ACADEMY.to_string(), scripts_of(ACADEMY_USES));
    scripts.insert(DEAN.to_string(), dean_script());
    register_scripts(scripts);
}

/// p1's main phase with `field`; both hands hold a card, so no turn ends by itself.
fn board(seed: &str, field: Value) -> Scenario {
    register();
    let mut s = scenario(json!({
        "seed": seed,
        "p1": { "hand": [SCHOOL], "field": field, "library": [SCHOOL, SCHOOL] },
        "p2": { "hand": [SCHOOL], "library": [SCHOOL, SCHOOL] },
    }));
    s.state_mut().players.p1.auto_end_turn = Some(false);
    s.state_mut().players.p2.auto_end_turn = Some(false);
    s
}

/// The effect as a card's list would run it, p1 its controller, then the settle; its events.
fn apply(s: &mut Scenario, effect: Effect) -> Vec<GameEvent> {
    let state = s.state_mut();
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        {
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(P1),
                    ..Default::default()
                },
            );
            (effect.apply)(&mut ctx);
        }
        settle(&mut sink, SettleOptions::default());
    }
    state.rng_cursor = rng.cursor();
    events
}

fn mana(s: &Scenario) -> i32 {
    s.state().players.p1.mana.current
}

fn why_not(s: &Scenario, id: &str) -> Option<String> {
    why_cannot_activate_ability(s.state(), P1, id, None)
        .err()
        .map(|error| error.message)
}

mod r2521003_run_activation_spends_a_use {
    use super::*;

    #[test]
    fn r2521003_it_runs_while_the_ability_has_a_use_and_spends_it() {
        let mut s = board("r2521003-spend", json!([{ "def": SCHOOL, "lane": 1 }]));
        let school = s.unit_at(P1, 1).id;
        let before = mana(&s);
        let events = apply(&mut s, run_of(&school, true));
        assert_eq!(mana(&s), before + GAIN);
        assert!(
            events
                .iter()
                .all(|event| !matches!(event, GameEvent::Activated { .. })),
            "no activation of the card's own"
        );
        // Its one use is spent: a second run finds none, and its own Activate is refused.
        apply(&mut s, run_of(&school, true));
        assert_eq!(mana(&s), before + GAIN);
        assert_eq!(why_not(&s, &school).as_deref(), Some(SPENT));
        s.expect_refused_with(|s| s.activate(school.as_str(), json!({})), SPENT);
    }

    #[test]
    fn r2521003_an_ability_its_card_used_itself_is_passed_by() {
        let mut s = board("r2521003-used", json!([{ "def": SCHOOL, "lane": 1 }]));
        let school = s.unit_at(P1, 1).id;
        let before = mana(&s);
        s.activate(school.as_str(), json!({}));
        assert_eq!(mana(&s), before + GAIN);
        apply(&mut s, run_of(&school, true));
        assert_eq!(mana(&s), before + GAIN);
    }

    #[test]
    fn r2521003_an_activate_n_runs_once_and_spends_every_use() {
        let mut s = board("r2521003-n", json!([{ "def": ACADEMY, "lane": 1 }]));
        let academy = s.unit_at(P1, 1).id;
        let before = mana(&s);
        apply(&mut s, run_of(&academy, true));
        assert_eq!(mana(&s), before + GAIN);
        apply(&mut s, run_of(&academy, true));
        assert_eq!(
            mana(&s),
            before + GAIN,
            "a card it ran is Activated once that turn"
        );
        assert_eq!(
            why_not(&s, &academy),
            Some(format!(
                "that ability has been used {ACADEMY_USES} times this turn"
            ))
        );
    }

    #[test]
    fn r2521003_r1876000_a_card_refused_its_own_activate_still_runs_once() {
        let mut s = board("r2521003-refused", json!([{ "def": SCHOOL, "lane": 1 }]));
        let school = s.unit_at(P1, 1).id;
        apply(
            &mut s,
            refuse_own_activate(ActivationOfArgs {
                instance_id: Some(school.clone()),
                ..ActivationOfArgs::default()
            }),
        );
        let before = mana(&s);
        apply(&mut s, run_of(&school, true));
        assert_eq!(mana(&s), before + GAIN);
        apply(&mut s, run_of(&school, true));
        assert_eq!(mana(&s), before + GAIN);
    }

    #[test]
    fn r2521003_a_second_dean_the_same_turn_runs_nothing_the_first_ran() {
        let mut s = board(
            "r2521003-deans",
            json!([
                { "def": DEAN, "lane": 1 },
                { "def": DEAN, "lane": 2 },
                { "def": SCHOOL, "lane": 3 },
                { "def": ACADEMY, "lane": 4 },
            ]),
        );
        let first = s.unit_at(P1, 1).id;
        let second = s.unit_at(P1, 2).id;
        let school = s.unit_at(P1, 3).id;
        let academy = s.unit_at(P1, 4).id;
        let before = mana(&s);
        s.activate(first.as_str(), json!({}));
        // The school's one use, and the academy once, its two uses spent.
        assert_eq!(mana(&s), before + 2 * GAIN);
        s.activate(second.as_str(), json!({}));
        // Each was Activated this turn, so the second dean runs neither.
        assert_eq!(mana(&s), before + 2 * GAIN);
        assert_eq!(why_not(&s, &school).as_deref(), Some(SPENT));
        assert_eq!(
            why_not(&s, &academy),
            Some(format!(
                "that ability has been used {ACADEMY_USES} times this turn"
            ))
        );
    }

    #[test]
    fn r2521003_the_next_turn_starts_the_count_afresh() {
        let mut s = board("r2521003-next", json!([{ "def": SCHOOL, "lane": 1 }]));
        let school = s.unit_at(P1, 1).id;
        apply(&mut s, run_of(&school, true));
        s.end_turn();
        s.end_turn();
        assert_eq!(s.state().active, P1);
        assert_eq!(why_not(&s, &school), None);
        let before = mana(&s);
        apply(&mut s, run_of(&school, true));
        assert_eq!(mana(&s), before + GAIN);
        assert_eq!(why_not(&s, &school).as_deref(), Some(SPENT));
    }

    #[test]
    fn r1876001_r2521003_without_spend_use_no_use_is_spent() {
        let mut s = board("r2521003-free", json!([{ "def": SCHOOL, "lane": 1 }]));
        let school = s.unit_at(P1, 1).id;
        let before = mana(&s);
        apply(&mut s, run_of(&school, false));
        apply(&mut s, run_of(&school, false));
        assert_eq!(mana(&s), before + 2 * GAIN);
        assert_eq!(why_not(&s, &school), None);
        s.activate(school.as_str(), json!({}));
        assert_eq!(mana(&s), before + 3 * GAIN);
    }

    #[test]
    fn r2521003_spend_use_travels_as_spend_use_and_is_left_off_when_absent() {
        let args: ActivationOfArgs = json_as(json!({ "instanceId": "c1", "spendUse": true }));
        assert_eq!(args.spend_use, Some(true));
        assert_eq!(
            serde_json::to_value(&args).expect("serializes")["spendUse"],
            json!(true)
        );
        let wire = serde_json::to_value(ActivationOfArgs::default()).expect("serializes");
        assert!(wire.get("spendUse").is_none(), "{wire}");
    }
}
