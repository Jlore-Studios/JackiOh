//! A unit's "when this attacks" and "after this attacks" text in the AI's evaluation (R2688000): the
//! decision's root plays each visible unit's next attack beside the same attack by its hookless twin
//! (`attack_hook_prices`), and `unit_worth` adds `AI_EVAL.attack_hook` of the difference while the unit
//! may attack, for either side's units alike. A face-down or hand card is never probed, hidden cards
//! change no decision (R185), and a zero weight, the frozen baseline's, reads no price. Its twin keeps
//! every keyword the unit holds, a lent one among them, and a unit already Vanilla is its own twin
//! (R4811000). Frenzy is no attack hook: the twin frenzies as the unit does (R4799002).
//!
//! The cards: The Propagation's Wasp, (1) 3/3, "After this attacks, summon a copy of this, then evolve.
//! / Evolve: Gain Flying."; The Propagation's Hornet, (2) 3/4, Rush and the Wasp's text; Badger, (1)
//! 2/3, "When this attacks, gain a random Stinkyscape Resource."; Honey Badger, (2) 4/6, Rush and the
//! Badger's text; Tmack Trav, (3) 7/7 "Rush, Frenzy", Radiant 14/14 with Pierce too, and no script;
//! Polymorph, (3) "Transform a Unit into a Sheep Token"; Midrange Menace, (3) 9/9 Taunt; Mr. Vanilla, a
//! (1) 4/4 with no text (`plain`); the (1) Traps Sheepish and My Pawn; and a test card, Prem Panther
//! printing Pierce (`PIERCING_PANTHER`).
//!
//! Below them, #5007's two scenes hold `AI_EVAL.attack_hook` at the 0.5 its screen of 1.0 left: a unit's
//! next attack counts half of what its text gives, the seat's units and the enemy's alike.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, register_cards, unit};

const WASP: &str = "stinkyscape-091";
const HORNET: &str = "stinkyscape-092";
const BADGER: &str = "stinkyscape-075";
const HONEY_BADGER: &str = "stinkyscape-076";
const TMACK: &str = "stinkyscape-094";
const POLYMORPH: &str = "meditative-024";
const MENACE: &str = "core-019";
const VANILLA: &str = "core-008";
const SHEEPISH: &str = "core-041";
const MY_PAWN: &str = "core-096";
const LIBRARY: &[&str] = &[VANILLA, VANILLA, VANILLA, VANILLA];

/// AI_EVAL with a unit's next attack priced in full: the arm #5007 screened.
const FULL: EvalWeights = EvalWeights {
    attack_hook: 1.0,
    ..AI_EVAL
};

/// AI_EVAL with a unit's next attack priced at nothing.
const UNPRICED: EvalWeights = EvalWeights {
    attack_hook: 0.0,
    ..AI_EVAL
};

/// The AI's turn with the AI's side `p1` and the human's `p2`, each with `LIBRARY` added.
fn board(seed: &str, mut p1: Value, mut p2: Value) -> GameState {
    register_cards();
    p1["library"] = json!(LIBRARY);
    p2["library"] = json!(LIBRARY);
    scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }))
        .state()
        .clone()
}

/// A Mr. Vanilla set to `attack`/`health`: a body with no text and no keyword, the size of the unit a
/// test weighs it against.
fn plain(attack: i32, health: i32) -> Value {
    json!({ "def": VANILLA, "statsOverride": { "attack": attack, "health": health } })
}

/// The AI's attack-hook prices on `state` and the nodes their probes spent.
fn priced(state: &GameState) -> (Vec<AttackHookPrice>, usize) {
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let prices = attack_hook_prices(state, AI, &counter);
    (prices, counter.used())
}

/// `attack_hooks` as a decision passes them, with no other price.
fn prices_of(attack_hooks: Vec<AttackHookPrice>) -> Prices {
    Prices {
        attack_hooks,
        ..Prices::default()
    }
}

/// The price of `controller`'s `def_id`, which must be priced.
fn price_of(prices: &[AttackHookPrice], def_id: &str, controller: PlayerId) -> f64 {
    prices
        .iter()
        .find(|entry| entry.def_id == def_id && entry.controller == controller)
        .map(|entry| entry.price)
        .unwrap_or_else(|| panic!("{controller}'s {def_id}'s attack hook is priced: {prices:?}"))
}

/// `a` and `b` agree to the last float rounding.
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} against {b}");
}

#[test]
fn r2688000_an_enemy_wasp_is_worth_more_to_remove_than_an_equal_vanilla() {
    let state = board(
        "ai-attack-hook-enemy",
        json!({}),
        json!({ "field": [WASP, plain(3, 3)] }),
    );
    let (wasp, vanilla) = (unit(&state, HUMAN, WASP), unit(&state, HUMAN, VANILLA));
    let unpriced_wasp = kill_worth(&state, wasp, &AI_EVAL, Prices::NONE);
    let unpriced_vanilla = kill_worth(&state, vanilla, &AI_EVAL, Prices::NONE);
    close(unpriced_wasp, unpriced_vanilla);
    let unpriced_eval = evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL);
    let (prices, used) = priced(&state);
    let price = price_of(&prices, WASP, HUMAN);
    assert!(price > 0.0, "the copy it makes counts: {price}");
    assert!(
        (3..=AI_ATTACK_HOOK_PROBE_NODES).contains(&used),
        "a turn end and two attacks at least: {used}"
    );
    let prices = prices_of(prices);
    let wasp_kill = kill_worth(&state, wasp, &AI_EVAL, &prices);
    close(wasp_kill - unpriced_wasp, AI_EVAL.attack_hook * price);
    let vanilla_kill = kill_worth(&state, vanilla, &AI_EVAL, &prices);
    assert!(
        wasp_kill > vanilla_kill,
        "the copy it makes each attack counts: {wasp_kill} against {vanilla_kill}"
    );
    let eval = evaluate_priced(&state, AI, NextSwing::Enemy, &AI_EVAL, &prices);
    assert!(
        eval < unpriced_eval,
        "an enemy Wasp counts for more against the AI: {eval} against {unpriced_eval}"
    );
}

#[test]
fn r2688000_given_one_removal_the_ai_polymorphs_the_wasp() {
    for (seed, field) in [
        ("ai-attack-hook-removal", json!([WASP, plain(3, 3)])),
        ("ai-attack-hook-removal-flipped", json!([plain(3, 3), WASP])),
    ] {
        let state = board(seed, json!({ "hand": [POLYMORPH] }), json!({ "field": field }));
        let wasp = unit(&state, HUMAN, WASP).id.clone();
        let polymorph = state.players[AI].hand[0].id.clone();
        let turn = play_ai_turn(&state, AI, &mut AiOptions::new(create_rng(seed, 0)));
        let targets = turn.actions.iter().find_map(|action| match &action.body {
            ActionBody::Play {
                instance_id, targets, ..
            } if *instance_id == polymorph => Some(targets.clone()),
            _ => None,
        });
        assert_eq!(
            targets,
            Some(Some(vec![Selection::Instance { instance_id: wasp }])),
            "{seed}: {:?}",
            turn.actions
        );
    }
}

#[test]
fn r2688000_an_own_badger_is_worth_more_than_a_vanilla_body_of_its_stats() {
    let state = board(
        "ai-attack-hook-own",
        json!({ "field": [BADGER, plain(2, 3)] }),
        json!({}),
    );
    let (badger, vanilla) = (unit(&state, AI, BADGER), unit(&state, AI, VANILLA));
    let unpriced_badger = unit_worth(&state, badger, &AI_EVAL, Prices::NONE);
    close(
        unpriced_badger,
        unit_worth(&state, vanilla, &AI_EVAL, Prices::NONE),
    );
    let (prices, used) = priced(&state);
    let price = price_of(&prices, BADGER, AI);
    assert!(price > 0.0, "the Resource it gains counts: {price}");
    assert!(
        (4..=AI_ATTACK_HOOK_PROBE_NODES).contains(&used),
        "two turn ends and two attacks at least: {used}"
    );
    let prices = prices_of(prices);
    let worth = unit_worth(&state, badger, &AI_EVAL, &prices);
    close(worth - unpriced_badger, AI_EVAL.attack_hook * price);
    let vanilla_worth = unit_worth(&state, vanilla, &AI_EVAL, &prices);
    assert!(
        worth > vanilla_worth,
        "the Resource it gains each attack counts: {worth} against {vanilla_worth}"
    );
}

#[test]
fn r2688000_a_zero_weight_reads_no_price() {
    const ZERO: EvalWeights = EvalWeights {
        attack_hook: 0.0,
        ..AI_EVAL
    };
    assert_eq!(GREEDY_EVAL.attack_hook, 0.0);
    let state = board(
        "ai-attack-hook-zero",
        json!({ "field": [BADGER] }),
        json!({ "field": [WASP] }),
    );
    let (prices, _) = priced(&state);
    assert!(
        price_of(&prices, BADGER, AI) > 0.0 && price_of(&prices, WASP, HUMAN) > 0.0,
        "{prices:?}"
    );
    let prices = prices_of(prices);
    let (badger, wasp) = (unit(&state, AI, BADGER), unit(&state, HUMAN, WASP));
    for w in [&ZERO, &GREEDY_EVAL] {
        assert_eq!(
            evaluate_priced(&state, AI, NextSwing::Enemy, w, &prices),
            evaluate_priced(&state, AI, NextSwing::Enemy, w, Prices::NONE)
        );
        for card in [badger, wasp] {
            assert_eq!(
                unit_worth(&state, card, w, &prices),
                unit_worth(&state, card, w, Prices::NONE)
            );
            assert_eq!(
                kill_worth(&state, card, w, &prices),
                kill_worth(&state, card, w, Prices::NONE)
            );
        }
    }
}

/// A test card on top of the catalog: Core #32 Prem Panther, (3) 5/4 Rush, "Whenever this attacks, draw 1
/// for each Unit that attack destroyed", printing Pierce too, so its text gives nothing on a hit on the
/// hero and its Pierce takes that hit through the hero's Armor.
const PIERCING_PANTHER: &str = "ai-test-piercing-panther";

/// `board` with `PIERCING_PANTHER` registered for this thread: Prem Panther's script under its id.
fn panther_board(seed: &str, p1: Value, p2: Value) -> GameState {
    register_cards();
    let mut catalog = (*jackioh_cards::CATALOG).clone();
    let mut panther = catalog
        .get("core-032")
        .expect("Prem Panther is in the catalog")
        .clone();
    panther.id = PIERCING_PANTHER.to_string();
    panther.index = PIERCING_PANTHER.to_string();
    panther.base.keywords.push(Keyword::Pierce);
    panther.radiant.keywords.push(Keyword::Pierce);
    catalog.insert(PIERCING_PANTHER.to_string(), panther);
    register_catalog_as(catalog, jackioh_cards::catalog_version());
    let mut scripts = jackioh_cards::scripts_of();
    let script = scripts
        .get("core-032")
        .expect("Prem Panther has a script")
        .clone();
    scripts.insert(PIERCING_PANTHER.to_string(), script);
    register_scripts(scripts);
    let mut p1 = p1;
    let mut p2 = p2;
    p1["library"] = json!(LIBRARY);
    p2["library"] = json!(LIBRARY);
    scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }))
        .state()
        .clone()
}

#[test]
fn r2688000_the_twin_keeps_its_printed_keywords() {
    // The Panther's hit on the hero gives nothing through its text, so its price is 0 only if its twin
    // pierces the hero's 5 Armor as it does. (Radiant Tmack Trav was this card until Frenzy, its one
    // text, stopped counting as an attack hook, R4799002.)
    let state = panther_board(
        "ai-attack-hook-twin",
        json!({ "field": [PIERCING_PANTHER] }),
        json!({ "armor": 5 }),
    );
    let (prices, _) = priced(&state);
    close(price_of(&prices, PIERCING_PANTHER, AI), 0.0);
}

#[test]
fn r4799002_a_frenzy_unit_with_no_attack_text_is_never_probed() {
    // Tmack Trav prints Rush and Frenzy and no attack hook: its twin would frenzy as it does, so the probe
    // could only read 0, and it spends no node on it.
    let state = board(
        "ai-attack-hook-frenzy",
        json!({ "field": [TMACK] }),
        json!({ "field": [plain(1, 1)] }),
    );
    assert!(unit_has(&state, unit(&state, AI, TMACK), KeywordKind::Frenzy));
    assert_eq!(priced(&state), (vec![], 0));
}

#[test]
fn r2688000_the_twin_is_scored_with_its_text_back() {
    // A Vanilla unit takes no keyword its controller's "your Units have X" lends (R1859007), so a lent
    // Poisonous moves the price only if the twin were scored still Vanilla.
    let state = board(
        "ai-attack-hook-text-back",
        json!({ "field": [BADGER] }),
        json!({}),
    );
    let mut lent = state.clone();
    lent.players[AI].mods.push(PlayerModifier {
        id: "ai-attack-hook-lent".to_string(),
        expiry: ModifierExpiry::Never,
        kind: ModifierKind::UnitsHaveKeyword {
            keyword: Keyword::Poisonous,
            from_spell: None,
        },
    });
    assert!(unit_has(&lent, unit(&lent, AI, BADGER), KeywordKind::Poisonous));
    let (bare, _) = priced(&state);
    let (with_lent, _) = priced(&lent);
    close(price_of(&with_lent, BADGER, AI), price_of(&bare, BADGER, AI));
}

#[test]
fn r2688000_a_taunt_takes_the_attack_the_hero_cannot() {
    let state = board(
        "ai-attack-hook-taunt",
        json!({ "field": [BADGER] }),
        json!({ "field": [MENACE] }),
    );
    let (prices, _) = priced(&state);
    assert!(
        price_of(&prices, BADGER, AI) > 0.0,
        "it gains its Resource before it dies on the Taunt: {prices:?}"
    );
}

/// `state` with "your Units have `keyword`" on the AI's side, as a War Doctrine leaves it (R1859007).
fn with_lent(state: &GameState, keyword: Keyword) -> GameState {
    let mut lent = state.clone();
    lent.players[AI].mods.push(PlayerModifier {
        id: "ai-attack-hook-lent".to_string(),
        expiry: ModifierExpiry::Never,
        kind: ModifierKind::UnitsHaveKeyword {
            keyword,
            from_spell: None,
        },
    });
    lent
}

#[test]
fn r4811000_a_lent_poisonous_moves_no_attack_hook_price_against_a_taunt() {
    // The Badger dies on the 9/9 Taunt either way; with Poisonous lent it takes the Menace with it, and
    // so does its twin.
    let bare = board(
        "ai-attack-hook-lent-poisonous",
        json!({ "field": [BADGER], "hand": [VANILLA] }),
        json!({ "hand": [VANILLA], "field": [MENACE] }),
    );
    let lent = with_lent(&bare, Keyword::Poisonous);
    assert!(unit_has(&lent, unit(&lent, AI, BADGER), KeywordKind::Poisonous));
    close(
        price_of(&priced(&lent).0, BADGER, AI),
        price_of(&priced(&bare).0, BADGER, AI),
    );
}

#[test]
fn r4811000_a_lent_trample_moves_no_attack_hook_price_against_a_taunt() {
    // 1 of the Badger's 2 Attack tramples over the 1/1 Taunt to the hero, from its twin too.
    let mut bare = board(
        "ai-attack-hook-lent-trample",
        json!({ "field": [BADGER] }),
        json!({ "field": [plain(1, 1)] }),
    );
    let guard = unit(&bare, HUMAN, VANILLA).id.clone();
    find_instance_mut(&mut bare, &guard)
        .expect("the 1/1")
        .granted_keywords
        .push(Keyword::Taunt);
    let lent = with_lent(&bare, Keyword::Trample);
    assert!(unit_has(&lent, unit(&lent, AI, BADGER), KeywordKind::Trample));
    close(
        price_of(&priced(&lent).0, BADGER, AI),
        price_of(&priced(&bare).0, BADGER, AI),
    );
}

#[test]
fn r4811000_a_vanilla_unit_that_holds_frenzy_has_no_text_to_price() {
    // A Vanilla unit runs no text, and Frenzy is no attack hook (R4799002): nothing is probed.
    let mut state = board(
        "ai-attack-hook-vanilla-frenzy",
        json!({ "field": [MENACE] }),
        json!({}),
    );
    let menace = unit(&state, AI, MENACE).id.clone();
    let card = find_instance_mut(&mut state, &menace).expect("the Menace");
    card.vanilla = true;
    card.granted_keywords.push(Keyword::Frenzy);
    assert_eq!(priced(&state), (vec![], 0));
}

#[test]
fn r2688000_a_board_without_an_attack_hook_prices_nothing_and_spends_no_node() {
    let state = board(
        "ai-attack-hook-none",
        json!({ "hand": [BADGER], "field": [VANILLA] }),
        json!({ "hand": [WASP], "backrow": [SHEEPISH], "field": [VANILLA] }),
    );
    assert_eq!(priced(&state), (vec![], 0));
}

#[test]
fn r2688000_hidden_cards_change_no_price_and_no_decision() {
    let a = board(
        "ai-attack-hook-r185",
        json!({ "hand": [POLYMORPH], "field": [BADGER] }),
        json!({ "hand": [VANILLA], "backrow": [SHEEPISH], "field": [WASP, plain(3, 3)] }),
    );
    let mut b = a.clone();
    b.players[HUMAN].hand[0].def_id = BADGER.to_string();
    b.players[HUMAN]
        .backrow
        .iter_mut()
        .flatten()
        .next()
        .expect("the enemy's Trap is set")
        .def_id = MY_PAWN.to_string();
    // The probe world as `decide` builds it: the seat's view, determinized on the probe seed.
    let probe_of = |state: &GameState| {
        let public = redact(state, AI);
        let ground = det_ground(&public, AI);
        determinize_on(
            &public,
            AI,
            &mut Rng::new(AI_SEARCH.probe_seed, 0),
            DeterminizeOptions::default(),
            &ground,
        )
    };
    let (on_a, on_b) = (priced(&probe_of(&a)), priced(&probe_of(&b)));
    assert!(price_of(&on_a.0, WASP, HUMAN) > 0.0, "{on_a:?}");
    assert_eq!(
        on_a, on_b,
        "the enemy's hand card and face-down Trap change no price and no node"
    );
    let decide_on = |state: &GameState| {
        decide(
            state,
            AI,
            &mut AiOptions::new(create_rng("ai-attack-hook-r185", 0)),
        )
        .expect("the AI acts")
    };
    let (on_a, on_b) = (decide_on(&a), decide_on(&b));
    assert_eq!(on_a.action, on_b.action);
    assert_eq!(on_a.line, on_b.line);
}

#[test]
fn r2688000_the_probes_stay_within_their_cap_and_the_decision_within_its_budget() {
    let state = board(
        "ai-attack-hook-budget",
        json!({ "field": [BADGER, { "def": BADGER, "radiant": true }, WASP, HONEY_BADGER] }),
        json!({ "field": [WASP, { "def": WASP, "radiant": true }, HORNET, HONEY_BADGER] }),
    );
    let (_, used) = priced(&state);
    assert!((3..=AI_ATTACK_HOOK_PROBE_NODES).contains(&used), "{used}");
    let decision = decide(
        &state,
        AI,
        &mut AiOptions::new(create_rng("ai-attack-hook-budget", 0)),
    )
    .expect("the AI acts");
    assert!(decision.stats.nodes <= AI_BUDGET.nodes, "{:?}", decision.stats);
}

/// #5007: an own Badger that may attack counts `AI_EVAL.attack_hook` of what its next attack's Resource
/// gives (R2688000), the 0.5 its screen of 1.0 left: half the price over a plain 2/3. At 1.0 it would
/// count the whole price, and at 0 nothing.
#[test]
fn r2688000_an_own_badger_counts_half_of_its_next_attack() {
    let state = board(
        "ai-attack-hook-own",
        json!({ "field": [BADGER, plain(2, 3)] }),
        json!({}),
    );
    let (badger, body) = (unit(&state, AI, BADGER), unit(&state, AI, VANILLA));
    let (prices, _) = priced(&state);
    let price = price_of(&prices, BADGER, AI);
    assert!(price > 0.0, "the Resource it gains counts: {price}");
    let prices = prices_of(prices);
    let over =
        |w: &EvalWeights| unit_worth(&state, badger, w, &prices) - unit_worth(&state, body, w, &prices);
    close(over(&AI_EVAL), price / 2.0);
    close(over(&FULL), price);
    close(over(&UNPRICED), 0.0);
}

/// #5007: destroying an enemy Wasp is worth `AI_EVAL.attack_hook` of the copy its next attack makes
/// (R2688000), the 0.5 its screen of 1.0 left: half the price over destroying a plain 3/3, which the
/// reply's trader ranks by (`kill_worth`). At 1.0 it would be the whole price, and at 0 nothing.
#[test]
fn r2688000_destroying_an_enemy_wasp_is_worth_half_of_its_next_copy() {
    let state = board(
        "ai-attack-hook-enemy",
        json!({}),
        json!({ "field": [WASP, plain(3, 3)] }),
    );
    let (wasp, body) = (unit(&state, HUMAN, WASP), unit(&state, HUMAN, VANILLA));
    let (prices, _) = priced(&state);
    let price = price_of(&prices, WASP, HUMAN);
    assert!(price > 0.0, "the copy it makes counts: {price}");
    let prices = prices_of(prices);
    let over = |w: &EvalWeights| kill_worth(&state, wasp, w, &prices) - kill_worth(&state, body, w, &prices);
    close(over(&AI_EVAL), price / 2.0);
    close(over(&FULL), price);
    close(over(&UNPRICED), 0.0);
}
