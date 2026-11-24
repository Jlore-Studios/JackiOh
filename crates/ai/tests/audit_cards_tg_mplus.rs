//! Audit (Training Grounds and Meditative+ cards): exploratory probes.

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

fn dump_hand(s: &Scenario, p: PlayerId) -> Vec<(String, bool, Option<i32>, i32)> {
    s.hand(p)
        .iter()
        .map(|c| (c.def_id.clone(), c.radiant, c.cost_override, c.cost_mod))
        .collect()
}

/// Probe 1: Dogpile answers a Spell that exiles itself (Legacy: True Strike).
#[test]
fn probe_dogpile_after_true_strike() {
    const DOGPILE: &str = "meditativeplus-017";
    const STRIKE: &str = "traininggrounds-058";
    let mut s = game(json!({
        "seed": "p1",
        "p1": { "hand": [STRIKE, DOGPILE], "library": [FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
    }));
    let before = hp(&s, P2);
    s.play(STRIKE, pick::targets(pick::hero("p2")));
    println!("P1 hp loss on p2: {}", before - hp(&s, P2));
    println!("hand: {:?}", dump_hand(&s, P1));
    assert_eq!(before - hp(&s, P2), 6, "True Strike 4 + Dogpile 2");
}

/// Probe 2: Wondrous Wand draws Cognitive Offloading.
#[test]
fn probe_wand_draws_offloading() {
    const WAND: &str = "meditativeplus-003";
    const OFFLOADING: &str = "meditativeplus-018";
    const A: &str = "stand-in (9) 1/1";
    const B: &str = "stand-in (8) 2/2";
    let mut s = game(json!({
        "seed": "p2",
        "p1": { "hand": [{ "def": WAND, "radiant": true }], "library": [OFFLOADING, A, B, FILLER, FILLER, FILLER, FILLER], "mana": 1 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
    }));
    s.play(WAND, json!({}));
    println!("active {:?} turn {} phase {:?}", s.state().active, s.state().turn, s.state().phase);
    println!("p1 hand: {:?}", dump_hand(&s, P1));
    println!("p1 field: {:?}", active_units_of(s.state(), P1).iter().map(|c| c.def_id.clone()).collect::<Vec<_>>());
    for e in s.events() {
        let t = e.event_type().as_str().to_string();
        if t.contains("ost") || t.contains("rawn") || t.contains("layed") || t.contains("utopilot") {
            println!("  ev {}", serde_json::to_string(e).unwrap());
        }
    }
}

/// Probe 3: Radiant Bloodbloom gives KY's Competition Echo 1.
#[test]
fn probe_bloodbloom_echo_kys_competition() {
    const BLOOM: &str = "meditativeplus-006";
    const KY: &str = "meditativeplus-020";
    let mut s = game(json!({
        "seed": "p3",
        "p1": { "hand": [{ "def": BLOOM, "radiant": true }, KY, FILLER], "library": [FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
    }));
    s.play(BLOOM, json!({}));
    s.play(KY, json!({}));
    let mut asked = 0;
    while let Some(p) = s.state().pending.clone() {
        asked += 1;
        if asked > 80 {
            break;
        }
        let key = answer_key_of(&p.resume.data).expect("key");
        // two right answers then Stop each sequence
        let streak = p.streak.unwrap_or(0);
        if streak < 2 {
            s.answer(json!(format!("mode:{key}")));
        } else {
            let wrong = ["A", "B", "C", "D"].into_iter().find(|l| *l != key).unwrap();
            s.answer(json!(format!("mode:{wrong}")));
        }
    }
    let kennies: Vec<(i32, i32)> = active_units_of(s.state(), P1)
        .into_iter()
        .map(|c| {
            let v = unit_view(s.state(), c);
            (v.attack, v.health)
        })
        .collect();
    println!("asked {asked} kennies {kennies:?} hp {}", hp(&s, P1));
    assert_eq!(kennies, vec![(2, 2), (2, 2)], "Echo 1: the Spell resolves twice");
}

/// Probe 4: Radiant Bloodbloom + Legacy Reminisce.
#[test]
fn probe_bloodbloom_echo_reminisce() {
    const BLOOM: &str = "meditativeplus-006";
    const REM: &str = "traininggrounds-055";
    const A: &str = "stand-in (3) 9/9";
    const B: &str = "stand-in (4) 7/7";
    const C: &str = "stand-in (2) 2/2";
    let mut s = game(json!({
        "seed": "p4",
        "p1": { "hand": [{ "def": BLOOM, "radiant": true }, REM, FILLER], "graveyard": [A, B, C], "library": [FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
    }));
    s.play(BLOOM, json!({}));
    s.play(REM, json!({}));
    let mut n = 0;
    while let Some(p) = s.state().pending.clone() {
        n += 1;
        if n > 5 {
            break;
        }
        println!("prompt {} options {:?}", p.prompt, p.options);
        let first = p.options[0].key.clone();
        s.answer(json!(first));
    }
    println!("prompts {n} hand {:?}", dump_hand(&s, P1));
    println!("exile {:?}", s.pile(P1, "exile").iter().map(|c| c.def_id.clone()).collect::<Vec<_>>());
    println!("gy {:?}", s.pile(P1, "graveyard").iter().map(|c| c.def_id.clone()).collect::<Vec<_>>());
}

/// Probe 5: Radiant Bloodbloom + In a hurry (Echo).
#[test]
fn probe_bloodbloom_echo_in_a_hurry() {
    const BLOOM: &str = "meditativeplus-006";
    const HURRY: &str = "traininggrounds-015";
    let mut s = game(json!({
        "seed": "p5",
        "p1": { "hand": [{ "def": BLOOM, "radiant": true }, HURRY, FILLER], "library": [FILLER, FILLER, FILLER], "mana": 10, "health": 30 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
    }));
    s.play(BLOOM, json!({}));
    s.play(HURRY, json!({}));
    println!("units {}", active_units_of(s.state(), P1).len());
    s.end_turn();
    println!("units after end {}", active_units_of(s.state(), P1).len());
    assert_eq!(active_units_of(s.state(), P1).len(), 0);
}

/// Probe 6: Cognitive Offloading played from the hand under a Radiant Bloodbloom (Echo 1).
#[test]
fn probe_offloading_from_hand_with_echo() {
    const BLOOM: &str = "meditativeplus-006";
    const OFFLOADING: &str = "meditativeplus-018";
    let mut s = game(json!({
        "seed": "p6",
        "p1": { "hand": [{ "def": BLOOM, "radiant": true }, OFFLOADING, "stand-in (1) 4/4"], "library": [FILLER, FILLER, FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
    }));
    s.play(BLOOM, json!({}));
    s.play(OFFLOADING, json!({}));
    println!("active {:?} turn {} hp {} hand {:?}", s.state().active, s.state().turn, hp(&s, P1), dump_hand(&s, P1));
}

/// Probe 7: Spirit Lash by Flashback under a Garden.
#[test]
fn probe_spirit_lash_flashback_under_garden() {
    const LASH: &str = "meditativeplus-005";
    const GARDEN: &str = "meditativeplus-022";
    let s = game(json!({
        "seed": "p7",
        "p1": { "hand": [FILLER], "graveyard": [LASH], "backrow": [{ "def": GARDEN, "faceUp": true }], "library": [FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "field": ["stand-in (1) 4/4"], "library": [FILLER, FILLER, FILLER] },
    }));
    for a in s.offered(P1) {
        println!("offered {}", serde_json::to_string(&a).unwrap());
    }
}

/// Probe 8: KY's Intuition's offer in both views.
#[test]
fn probe_kys_intuition_views() {
    const KYI: &str = "meditativeplus-025";
    let mut s = game(json!({
        "seed": "p8",
        "p1": { "hand": [KYI, FILLER], "library": [FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
    }));
    s.play(KYI, json!({}));
    let pending = s.state().pending.clone().expect("prompt");
    println!("state options: {:?}", pending.options.iter().map(|o| (o.key.clone(), o.label.chars().take(40).collect::<String>())).collect::<Vec<_>>());
    println!("resume data keys: {:?}", pending.resume.data.keys().collect::<Vec<_>>());
    println!("resume data: {}", serde_json::to_string(&pending.resume.data).unwrap().chars().take(600).collect::<String>());
    let v1 = serde_json::to_string(&s.view(P1)).unwrap();
    let v2 = serde_json::to_string(&s.view(P2)).unwrap();
    let at = v1.find("\"pending\"").unwrap_or(0);
    println!("p1 view pending: {}", v1[at..].chars().take(900).collect::<String>());
    let at2 = v2.find("\"pending\"").unwrap_or(0);
    println!("p2 view pending: {}", v2[at2..].chars().take(500).collect::<String>());
}

/// Probe 9: Level Up! (Radiant) and a Trigor chain body with a stats override.
#[test]
fn probe_trigor_radiant_chain() {
    const TRIGOR: &str = "traininggrounds-025";
    const KILL: &str = "traininggrounds-046";
    let mut s = game(json!({
        "seed": "p9",
        "p1": { "hand": [KILL, KILL, KILL], "field": [{ "def": TRIGOR, "radiant": true, "lane": 1 }], "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER] },
    }));
    let t = s.unit_at(P1, 1).id;
    s.play(KILL, pick::targets(pick::id(&t)));
    let units: Vec<_> = active_units_of(s.state(), P1).into_iter().map(|c| { let v = unit_view(s.state(), c); (c.def_id.clone(), c.radiant, v.attack, v.health) }).collect();
    println!("after radiant death: {units:?}");
    let t2 = s.unit_at(P1, 1).id;
    s.play(KILL, pick::targets(pick::id(&t2)));
    let units: Vec<_> = active_units_of(s.state(), P1).into_iter().map(|c| { let v = unit_view(s.state(), c); (c.def_id.clone(), c.radiant, v.attack, v.health) }).collect();
    println!("same turn second death (chain mark): {units:?}");
}

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

/// Probe X1: R673 on the Training Grounds Call's ninth entry.
#[test]
fn probe_glitch_odds_reach_the_ninth_entrys_call() {
    let seed = seed_where("audit-ctc-tg", |seed| rolled(&ctc_game(seed)) == vec![ALTERNATE.to_string()]);
    let mut s = ctc_game(&seed);
    println!("seed {seed} rolled {:?}", rolled(&s));
    s.state_mut().system_plays = Some(10000);
    s.end_turn();
    s.end_turn();
    println!("active {:?} hand {:?}", s.state().active, dump_hand(&s, P1));
    let added: Vec<String> = s.hand(P1).iter().map(|c| c.def_id.clone()).filter(|d| d != FILLER).collect();
    println!("added {added:?}");
    assert_eq!(added, vec![GLITCH_DEF_ID.to_string()], "R673: every card generated into a hand, added from a pool");
}

/// Control for X1: Recruitment Posters' start-of-turn add under the same odds.
#[test]
fn probe_glitch_odds_reach_recruitment_posters() {
    const POSTERS: &str = "traininggrounds-020";
    let mut s = game(json!({
        "seed": "audit-posters",
        "p1": { "hand": [], "backrow": [{ "def": POSTERS, "faceUp": true }], "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER] },
    }));
    s.state_mut().system_plays = Some(10000);
    s.end_turn();
    s.end_turn();
    let added: Vec<String> = s.hand(P1).iter().map(|c| c.def_id.clone()).filter(|d| d != FILLER).collect();
    println!("posters added {added:?}");
    assert_eq!(added, vec![GLITCH_DEF_ID.to_string()]);
}

/// Probe Y5: the autopilot plays a hand of prompt-heavy cards.
#[test]
fn probe_autopilot_plays_prompt_cards() {
    const OFFLOADING: &str = "meditativeplus-018";
    for (i, hand) in [
        json!(["meditativeplus-020", "meditativeplus-026", "meditativeplus-016", "meditativeplus-025"]),
        json!(["meditativeplus-001", "traininggrounds-055", "traininggrounds-050", "meditativeplus-004"]),
        json!([{ "def": "meditativeplus-024", "radiant": true }, "meditativeplus-024", "traininggrounds-037", "meditativeplus-002"]),
        json!([{ "def": "meditativeplus-020", "radiant": true }, "meditativeplus-016-1", "meditativeplus-006", "traininggrounds-039"]),
        json!(["meditativeplus-019", "meditativeplus-010", "traininggrounds-057", "traininggrounds-049", "meditativeplus-017", "traininggrounds-058"]),
    ]
    .into_iter()
    .enumerate()
    {
        let mut s = game(json!({
            "seed": format!("auto-{i}"),
            "active": "p2",
            "p1": { "hand": hand, "graveyard": ["stand-in (3) 9/9", FILLER], "field": ["stand-in (1) 4/4"],
                    "library": [OFFLOADING, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER], "mana": 10 },
            "p2": { "hand": [FILLER, FILLER], "field": ["stand-in (2) 2/2", "stand-in (1) 3/3 Rush, First Strike"], "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER] },
        }));
        s.end_turn();
        let played: Vec<String> = s
            .events()
            .iter()
            .filter_map(|e| match e {
                GameEvent::CardPlayed { player, def_id, .. } if *player == P1 => Some(def_id.clone()),
                _ => None,
            })
            .collect();
        println!("game {i}: active {:?} turn {} pending {:?} result {:?} played {played:?}", s.state().active, s.state().turn, s.state().pending.as_ref().map(|p| (p.player_id, p.prompt.clone())), s.state().result.is_some());
        println!("   p1 hand {:?}", s.hand(P1).iter().map(|c| c.def_id.clone()).collect::<Vec<_>>());
    }
}

const PALANTIR: &str = "classic-004";
const MINE: &str = "stand-in (1) 3/3";
const THEIRS: &str = "stand-in (1) 4/4";

/// P2's Palantir ("When your opponent plays a (2) Cost Spell, Tribute this to steal and cast it") steals
/// the (2) Cost Spell P1 plays at P1's own Unit and casts it for P2.
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
/// legal"; Palantir's stolen cast is "aimed in your favor" (R831001, R452, R656).
#[test]
fn r656_fortify_cast_by_a_cast_that_targets_enemies_lands_on_its_casters_unit() {
    const FORTIFY: &str = "traininggrounds-012";
    let (s, mine, theirs) = stolen_by_palantir(json!({ "def": FORTIFY, "costMod": 1 }));
    assert!(s.pile(P2, "graveyard").iter().any(|card| card.def_id == FORTIFY), "P2 stole and cast it");
    let (mine, theirs) = (unit_view(s.state(), s.card(mine.as_str())), unit_view(s.state(), s.card(theirs.as_str())));
    println!("p1's unit {}/{} p2's unit {}/{}", mine.attack, mine.max_health, theirs.attack, theirs.max_health);
    assert_eq!(theirs.max_health, 9, "the caster's own Unit takes the +5 Health");
    assert_eq!(mine.max_health, 3, "the caster's enemy takes no buff");
}

/// TG #27 Hopeful Jelly Bean: "Make a permanent Radiant." A buff, aimed the same way.
#[test]
fn r656_hopeful_jelly_bean_cast_by_a_cast_that_targets_enemies_lands_on_its_casters_permanent() {
    const BEAN: &str = "traininggrounds-027";
    let (s, mine, theirs) = stolen_by_palantir(json!({ "def": BEAN, "costMod": -2 }));
    assert!(s.pile(P2, "graveyard").iter().any(|card| card.def_id == BEAN), "P2 stole and cast it");
    println!("p1's unit radiant {} p2's unit radiant {}", s.card(mine.as_str()).radiant, s.card(theirs.as_str()).radiant);
    assert!(s.card(theirs.as_str()).radiant, "the caster's own Unit becomes Radiant");
    assert!(!s.card(mine.as_str()).radiant, "the caster's enemy does not");
}

/// Control: TG #59 Legacy: Fig of Life declares `aim: help`, and the same stolen cast heals the caster's side.
#[test]
fn control_fig_of_life_stolen_by_palantir_heals_the_casters_side() {
    const FIG: &str = "traininggrounds-059";
    let (s, _mine, _theirs) = stolen_by_palantir(json!({ "def": FIG, "costMod": -1 }));
    assert!(s.pile(P2, "graveyard").iter().any(|card| card.def_id == FIG), "P2 stole and cast it");
    println!("p1 hp {} p2 hp {}", hp(&s, P1), hp(&s, P2));
    assert_eq!(hp(&s, P1), 30, "never the caster's enemy");
}
