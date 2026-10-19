//! The Meditative+ set (issue #1173) meets the AI (issue #1186): its health price, its declared modes
//! and its two prices need nothing new here, since `legal_actions` lists every play they allow and the
//! search tries each through `reduce`. Bloodbloom's health price (R1180004) offers a Spell only the
//! health affords, and the AI casts it for lethal with no mana but never pays its last health for it.
//! Bob's four modes (R1185004) are four plays, the take with its target and only while there is an
//! enemy Unit to take, and the Radiant face's Discover pauses its list for the AI to answer. Supply
//! Drop's two modes come at both prices (R1181001), and Mountaintop Garden's dearer choice (R1181002)
//! is weighed at the normal price plus the premium.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, is_legal, register_cards, scenario};

const WONDROUS_WAND: &str = "meditativeplus-003";
const SUPPLY_DROP: &str = "meditativeplus-004";
const BLOODBLOOM: &str = "meditativeplus-006";
const GARDEN: &str = "meditativeplus-022";
const BOB: &str = "meditativeplus-024";
/// Core #35 Lunar Eclipse: (1) Spell, "Deal 3 damage" to a target: 3 health under Bloodbloom's price.
const ECLIPSE: &str = "core-035";
/// Core #8 Mr. Vanilla: a (1) 4/4 with no text, the hand, library and field filler.
const VANILLA: &str = "core-008";
/// Bob's modes, as its script declares them (R81).
const BOB_MODES: [&str; 4] = ["freeze", "take", "discover", "copies"];
const TAKE: &str = "take";

fn library() -> Value {
    json!([VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA])
}

fn decide_at(state: &GameState, seed: &str) -> Decision {
    decide(state, AI, &mut AiOptions::new(create_rng(seed, 0))).expect("the AI acts")
}

fn is_play_of(action: &ActionBody, card: &str) -> bool {
    matches!(action, ActionBody::Play { instance_id, .. } if instance_id == card)
}

/// The instance id of the AI's hand card of `def_id`.
fn in_hand(state: &GameState, def_id: &str) -> String {
    state.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == def_id)
        .map(|card| card.id.clone())
        .expect("the card is in the AI's hand")
}

/// The modes and `embiggen` choices the AI's candidates play `card` with, sorted.
fn priced_plays(state: &GameState, card: &str) -> Vec<(Vec<String>, Option<bool>)> {
    let mut plays: Vec<(Vec<String>, Option<bool>)> = candidate_actions(state, AI)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::Play {
                instance_id,
                modes,
                embiggen,
                ..
            } if instance_id == card => Some((modes.unwrap_or_default(), embiggen)),
            _ => None,
        })
        .collect();
    plays.sort();
    plays.dedup();
    plays
}

fn priced(mode: Option<&str>, embiggen: Option<bool>) -> (Vec<String>, Option<bool>) {
    (
        mode.map(|mode| vec![mode.to_string()]).unwrap_or_default(),
        embiggen,
    )
}

// M+ #6 Bloodbloom: the health price (R1180004)

/// The AI holds Bloodbloom and Lunar Eclipse with no mana, at `health`, against a hero at
/// `enemy_health`; Bloodbloom is played, so the Eclipse's price is 3 health.
fn priced_in_health(seed: &str, health: i32, enemy_health: i32) -> (GameState, String) {
    register_cards();
    let mut s = scenario(json!({
        "seed": seed,
        "p1": { "hand": [BLOODBLOOM, ECLIPSE], "mana": 0, "health": health, "library": library() },
        "p2": { "hand": [VANILLA], "health": enemy_health, "library": library() },
    }));
    s.play(BLOODBLOOM, json!({}));
    let state = s.state().clone();
    let eclipse = in_hand(&state, ECLIPSE);
    assert_eq!(state.players[AI].mana.current, 0);
    assert!(
        legal_actions(&state, AI)
            .iter()
            .any(|action| is_play_of(action, &eclipse)),
        "only the health affords the Eclipse"
    );
    (state, eclipse)
}

#[test]
fn r1180004_the_ai_pays_health_for_a_lethal_spell_with_no_mana_left() {
    let _preview = preview_sets(&[SetName::MeditativePlus]);
    let (state, eclipse) = priced_in_health("ai-health-price-lethal", 30, 3);
    let decision = decide_at(&state, "ai-health-price-lethal");
    assert!(is_legal(&state, AI, &decision.action));
    assert!(
        matches!(
            &decision.action,
            ActionBody::Play { instance_id, targets: Some(targets), .. }
                if *instance_id == eclipse && targets.as_slice() == [Selection::Hero { player: HUMAN }]
        ),
        "the Eclipse at the enemy hero: {:?}",
        decision.action
    );
    let after = act(&state, AI, &decision.action);
    assert_eq!(after.players[AI].hero.health, 27, "3 times its cost, in health");
    assert_eq!(after.players[AI].mana.current, 0);
    assert_eq!(after.result.map(|result| result.winner), Some(Winner::P1));
}

#[test]
fn r1180004_the_ai_never_pays_its_last_health_for_a_spell() {
    let _preview = preview_sets(&[SetName::MeditativePlus]);
    let (state, eclipse) = priced_in_health("ai-health-price-last", 3, 30);
    let decision = decide_at(&state, "ai-health-price-last");
    assert!(is_legal(&state, AI, &decision.action));
    assert!(
        !is_play_of(&decision.action, &eclipse),
        "the Eclipse would pay the AI's last 3 health: {:?}",
        decision.action
    );
}

// M+ #24 Bob, Wisconsin's Strongest Soldier: four declared modes (R1185004)

/// The AI holds Bob (Radiant on `radiant`) and a filler; p2's field is `enemy_field`.
fn bob_game(seed: &str, radiant: bool, enemy_field: Value) -> Scenario {
    register_cards();
    scenario(json!({
        "seed": seed,
        "p1": { "hand": [{ "def": BOB, "radiant": radiant }, VANILLA], "library": library() },
        "p2": { "hand": [VANILLA], "field": enemy_field, "library": library() },
    }))
}

/// The modes and targets the AI's candidates play Bob with.
fn bob_plays(state: &GameState) -> Vec<(Vec<String>, Vec<Selection>)> {
    let bob = in_hand(state, BOB);
    candidate_actions(state, AI)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::Play {
                instance_id,
                modes,
                targets,
                ..
            } if instance_id == bob => Some((modes.unwrap_or_default(), targets.unwrap_or_default())),
            _ => None,
        })
        .collect()
}

#[test]
fn r1185004_the_ai_weighs_bobs_four_modes_as_four_plays_the_take_with_its_target() {
    let _preview = preview_sets(&[SetName::MeditativePlus]);
    let s = bob_game("ai-bob-modes", false, json!([{ "def": VANILLA, "lane": 2 }]));
    let enemy = s.unit(HUMAN, 2).expect("the enemy Unit").id;
    let state = s.state().clone();
    let plays = bob_plays(&state);
    for mode in BOB_MODES {
        assert!(
            plays.iter().any(|(modes, _)| *modes == [mode]),
            "{mode}: {plays:?}"
        );
    }
    let take = [Selection::Instance { instance_id: enemy }];
    assert!(
        plays
            .iter()
            .all(|(modes, targets)| (*modes == [TAKE]) == (*targets == take)),
        "the take, and it alone, declares the enemy Unit: {plays:?}"
    );
    let decision = decide_at(&state, "ai-bob-modes");
    assert!(is_legal(&state, AI, &decision.action));
}

#[test]
fn r1185004_with_no_enemy_unit_the_ai_is_offered_no_take() {
    let _preview = preview_sets(&[SetName::MeditativePlus]);
    let s = bob_game("ai-bob-no-take", false, json!([]));
    let state = s.state().clone();
    let plays = bob_plays(&state);
    for mode in BOB_MODES.into_iter().filter(|mode| *mode != TAKE) {
        assert!(
            plays.iter().any(|(modes, _)| *modes == [mode]),
            "{mode}: {plays:?}"
        );
    }
    // R703: the take's target is needed, so with nothing to take it is not a play.
    assert!(plays.iter().all(|(modes, _)| *modes != [TAKE]), "{plays:?}");
    let decision = decide_at(&state, "ai-bob-no-take");
    assert!(is_legal(&state, AI, &decision.action));
}

#[test]
fn r1185004_the_ai_answers_the_radiant_bobs_discover_mid_list() {
    let _preview = preview_sets(&[SetName::MeditativePlus]);
    let mut s = bob_game("ai-bob-radiant", true, json!([{ "def": VANILLA, "lane": 1 }]));
    let enemy = s.unit(HUMAN, 1).expect("the enemy Unit").id;
    s.play(
        BOB,
        json!({ "zone": 1, "targets": [{ "pick": "instance", "instanceId": enemy }] }),
    );
    let state = s.state().clone();
    // Freeze and take done, the Discover asks; the draw and its copies wait behind it (R113).
    let pending = state.pending.clone().expect("the Discover is open");
    assert_eq!(pending.player_id, AI);
    assert!(state.players[AI].hand.iter().any(|card| card.id == enemy));
    let decision = decide_at(&state, "ai-bob-radiant");
    assert!(is_legal(&state, AI, &decision.action));
    let ActionBody::Answer { selection, .. } = &decision.action else {
        panic!("the AI answers the Discover: {:?}", decision.action);
    };
    let [Selection::Mode { option: pick }] = selection.as_slice() else {
        panic!("a Discover pick: {selection:?}");
    };
    let after = act(&state, AI, &decision.action);
    assert!(after.pending.is_none());
    let hand = &after.players[AI].hand;
    assert!(
        hand.iter().any(|card| card.def_id == *pick && card.radiant),
        "the pick, Radiant"
    );
    assert_eq!(
        hand.len() as i32,
        hand_cap_of(&after, AI),
        "the list resumed: the draw and its copies"
    );
}

// M+ #4 Supply Drop: two modes at two prices (R1181001)

fn supply_drop_game(seed: &str, mana: i32) -> GameState {
    register_cards();
    scenario(json!({
        "seed": seed,
        "p1": { "hand": [SUPPLY_DROP, VANILLA], "mana": mana, "library": library() },
        "p2": { "hand": [VANILLA], "field": [{ "def": VANILLA, "lane": 1 }], "library": library() },
    }))
    .state()
    .clone()
}

#[test]
fn r1181001_the_ai_weighs_both_supply_drop_modes_at_each_price_it_can_pay() {
    let _preview = preview_sets(&[SetName::MeditativePlus]);
    for (mana, embiggens) in [
        (5, vec![Some(false), Some(true)]),
        (4, vec![Some(false), Some(true)]),
        (3, vec![Some(false)]),
        (2, vec![Some(false)]),
    ] {
        let seed = format!("ai-supply-drop-{mana}");
        let state = supply_drop_game(&seed, mana);
        let drop = in_hand(&state, SUPPLY_DROP);
        let expected: Vec<(Vec<String>, Option<bool>)> = ["combat", "supplies"]
            .into_iter()
            .flat_map(|mode| {
                embiggens
                    .iter()
                    .map(move |embiggen| priced(Some(mode), *embiggen))
            })
            .collect();
        assert_eq!(priced_plays(&state, &drop), expected, "at {mana} mana");
        let decision = decide_at(&state, &seed);
        assert!(is_legal(&state, AI, &decision.action), "at {mana} mana");
    }
}

// M+ #22 Mountaintop Garden: the dearer choice (R1181002)

/// The AI holds Wondrous Wand (2, Embiggened 4) and a filler at `mana`, a Garden (Radiant on
/// `radiant`) face-up in its backrow.
fn garden_game(seed: &str, radiant: bool, mana: i32) -> GameState {
    register_cards();
    scenario(json!({
        "seed": seed,
        "p1": {
            "hand": [WONDROUS_WAND, VANILLA],
            "backrow": [{ "def": GARDEN, "lane": 1, "radiant": radiant }],
            "mana": mana,
            "library": library(),
        },
        "p2": { "hand": [VANILLA], "library": library() },
    }))
    .state()
    .clone()
}

#[test]
fn r1181002_the_ai_is_offered_the_gardens_dearer_choice_only_when_it_can_pay_the_premium() {
    let _preview = preview_sets(&[SetName::MeditativePlus]);
    // The base Garden's premium is 2 and the Radiant one's 1, over the Wand's normal (2): the Radiant
    // Garden's dearer choice costs 3, below the (4) the Wand prints.
    for (radiant, dearer) in [(false, 4), (true, 3)] {
        for mana in [dearer, dearer - 1] {
            let seed = format!("ai-garden-{radiant}-{mana}");
            let state = garden_game(&seed, radiant, mana);
            let wand = in_hand(&state, WONDROUS_WAND);
            let mut expected = vec![priced(None, Some(false))];
            if mana >= dearer {
                expected.push(priced(None, Some(true)));
            }
            assert_eq!(
                priced_plays(&state, &wand),
                expected,
                "Radiant Garden {radiant}, {mana} mana"
            );
            let decision = decide_at(&state, &seed);
            assert!(
                is_legal(&state, AI, &decision.action),
                "Radiant Garden {radiant}, {mana} mana"
            );
            if mana < dearer {
                continue;
            }
            let dearer_play = candidate_actions(&state, AI)
                .into_iter()
                .find(|action| {
                    matches!(action, ActionBody::Play { instance_id, embiggen: Some(true), .. } if *instance_id == wand)
                })
                .expect("the dearer choice is a candidate");
            let result = reduce(&state, &Action::new(dearer_play, AI, "ai-garden-dearer"));
            assert_eq!(result.error, None);
            let paid = result.events.iter().find_map(|event| match event {
                GameEvent::CardPlayed { cost_paid, .. } => Some(*cost_paid),
                _ => None,
            });
            assert_eq!(paid, Some(dearer), "the normal price plus the premium");
            // Radiant and Embiggened, the Wand draws 5: the filler and the five draws are in the hand.
            assert_eq!(result.state.players[AI].hand.len(), 6, "played Radiant");
        }
    }
}
