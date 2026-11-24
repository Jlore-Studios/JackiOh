//! Audit (Training Grounds and Meditative+ cards). Each failing test asserts the printed or ruled
//! behaviour; each `control_` test passes and shows the same setup doing the right thing for a card
//! that follows the rule.

use jackioh_engine::prelude::*;
use jackioh_engine::testkit::*;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;
const FILLER: &str = "core-005";

fn game(opts: serde_json::Value) -> Scenario {
    jackioh_cards::register_all();
    scenario(opts)
}

fn hp(s: &Scenario, p: PlayerId) -> i32 {
    s.state().players[p].hero.health
}

// Finding 1: TG #12 Fortify and TG #27 Hopeful Jelly Bean declare no `aim: help` (R656).

const PALANTIR: &str = "classic-004";
const MINE: &str = "stand-in (1) 3/3";
const THEIRS: &str = "stand-in (1) 4/4";

/// P2's Classic #4 Palantir ("When your opponent plays a (2) Cost Spell, Tribute this to steal and cast
/// it") steals the Spell P1 plays at P1's own Unit, its own cost seeded to (2), and casts it for P2:
/// R831001, "each target pick aimed by its declaration (R656: a harmful pick narrows to the caster's
/// enemies, a helpful one to their friends, when one is offered)".
fn stolen_by_palantir(spell: Value) -> (Scenario, String, String) {
    let mut s = game(json!({
        "seed": "audit-aim",
        "p1": { "hand": [spell], "field": [{ "def": MINE, "lane": 1 }], "library": [FILLER, FILLER, FILLER], "mana": 10 },
        "p2": {
            "hand": [FILLER],
            "field": [{ "def": THEIRS, "lane": 1 }],
            "backrow": [{ "def": PALANTIR, "faceUp": true }],
            "library": [FILLER, FILLER, FILLER],
        },
    }));
    let mine = s.unit_at(P1, 1).id;
    let theirs = s.unit_at(P2, 1).id;
    let card = s.hand(P1)[0].id.clone();
    s.play(card.as_str(), pick::targets(pick::id(&mine)));
    (s, mine, theirs)
}

/// TG #12 Fortify: "Give a Unit +5 Health, +1 Armor and Taunt." A buff. R656: "Every target declaration
/// says whether it harms ("harm") or helps ("help") ... a helpful one [narrows] to friends when one is
/// legal". The thief's cast must land on the thief's Unit, not on the Unit of the player it stole from.
#[test]
fn r656_fortify_cast_by_a_cast_that_targets_enemies_lands_on_its_casters_unit() {
    const FORTIFY: &str = "traininggrounds-012";
    let (s, mine, theirs) = stolen_by_palantir(json!({ "def": FORTIFY, "costMod": 1 }));
    assert!(s.pile(P2, "graveyard").iter().any(|card| card.def_id == FORTIFY), "P2 stole and cast it");
    let mine = unit_view(s.state(), s.card(mine.as_str()));
    let theirs = unit_view(s.state(), s.card(theirs.as_str()));
    println!("p1's unit {}/{}  p2's unit {}/{}", mine.attack, mine.max_health, theirs.attack, theirs.max_health);
    assert_eq!(theirs.max_health, 9, "the caster's own Unit takes the +5 Health");
    assert_eq!(mine.max_health, 3, "the caster's enemy takes no buff");
}

/// TG #27 Hopeful Jelly Bean: "Make a permanent Radiant." A buff, aimed the same way.
#[test]
fn r656_hopeful_jelly_bean_cast_by_a_cast_that_targets_enemies_lands_on_its_casters_permanent() {
    const BEAN: &str = "traininggrounds-027";
    let (s, mine, theirs) = stolen_by_palantir(json!({ "def": BEAN, "costMod": -2 }));
    assert!(s.pile(P2, "graveyard").iter().any(|card| card.def_id == BEAN), "P2 stole and cast it");
    println!("p1's unit radiant {}  p2's unit radiant {}", s.card(mine.as_str()).radiant, s.card(theirs.as_str()).radiant);
    assert!(s.card(theirs.as_str()).radiant, "the caster's own Unit becomes Radiant");
    assert!(!s.card(mine.as_str()).radiant, "the caster's enemy does not");
}

/// Control: TG #59 Legacy: Fig of Life ("Heal a target 20") declares `aim: help`, and the same stolen
/// cast heals the caster's side, never the player it was stolen from.
#[test]
fn control_fig_of_life_stolen_by_palantir_heals_the_casters_side() {
    const FIG: &str = "traininggrounds-059";
    let (s, mine, _theirs) = stolen_by_palantir(json!({ "def": FIG, "costMod": -1 }));
    assert!(s.pile(P2, "graveyard").iter().any(|card| card.def_id == FIG), "P2 stole and cast it");
    assert_eq!(hp(&s, P1), 30, "never the caster's enemy hero");
    assert_eq!(s.card(mine.as_str()).damage, 0);
}

// Finding 2: the Training Grounds Call's ninth entry adds its Call with no Glitch roll (R673).

const CTC_TG: &str = "traininggrounds-039";
const ALTERNATE: &str =
    "At the start of every turn of yours for the rest of the game, add a random Call to Chaos to your hand";

fn rolled(s: &Scenario) -> Vec<String> {
    s.events()
        .iter()
        .filter_map(|event| match event {
            GameEvent::ChaosRolled { effects, .. } => Some(effects.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

fn ctc_game(seed: &str) -> Scenario {
    let mut s = game(json!({
        "seed": seed,
        "p1": { "hand": [CTC_TG], "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER] },
    }));
    s.play(CTC_TG, json!({}));
    s
}

/// R673: "every card generated into a hand or a deck — added from a pool, shuffled in from one, ... — is
/// Glitch instead with odds n × GLITCH_ODDS_PER_SYSTEM_PLAY / GLITCH_ODDS_DENOMINATOR", n the match's
/// System plays. With n at the denominator the odds are certain, so the Call that TG #39's ninth entry
/// adds from the Call to Chaos pool at the start of its caster's next turn must be Glitch.
#[test]
fn r673_the_ninth_entrys_call_is_generated_into_a_hand_and_may_be_glitch() {
    let seed = seed_where("audit-ctc-tg", |seed| rolled(&ctc_game(seed)) == vec![ALTERNATE.to_string()]);
    let mut s = ctc_game(&seed);
    s.state_mut().system_plays = Some(GLITCH_ODDS_DENOMINATOR);
    s.end_turn();
    s.end_turn();
    let added: Vec<String> = s.hand(P1).iter().map(|c| c.def_id.clone()).filter(|d| d != FILLER).collect();
    println!("added at the start of P1's turn: {added:?}");
    assert_eq!(added, vec![GLITCH_DEF_ID.to_string()], "R673: a card added to a hand from a pool");
}

/// Control: TG #20 Recruitment Posters' start-of-turn add from a pool, under the same odds, is Glitch.
#[test]
fn control_recruitment_posters_start_of_turn_add_is_glitch_under_the_same_odds() {
    const POSTERS: &str = "traininggrounds-020";
    let mut s = game(json!({
        "seed": "audit-posters",
        "p1": { "hand": [], "backrow": [{ "def": POSTERS, "faceUp": true }], "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER] },
    }));
    s.state_mut().system_plays = Some(GLITCH_ODDS_DENOMINATOR);
    s.end_turn();
    s.end_turn();
    let added: Vec<String> = s.hand(P1).iter().map(|c| c.def_id.clone()).filter(|d| d != FILLER).collect();
    assert_eq!(added, vec![GLITCH_DEF_ID.to_string()]);
}
