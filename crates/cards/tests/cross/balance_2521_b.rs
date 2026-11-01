//! Issue #2521, "Patch: Stinkyscape balance changes I": behaviours B21 to B37 of its frozen spec
//! (`.fullsend/SPEC.md`), written by the spec-tester from that spec and the catalog alone, before any
//! script of the issue existed. Every test names the behavior it proves (`b21_…` to `b36_…`); about
//! half of them check a refusal, a fallback or a boundary. B37 (the AI's shadow ban) cannot be reached
//! from this crate, which does not depend on `jackioh-ai`; the gap note says so.
//!
//!  - B21 Meteor deals 15 on both faces, its Radiant face trampling on to the hero (§6.3, R1848001).
//!  - B22 Tmack Trav's extra attack after a kill on its Rush turn is never at the hero.
//!  - B23, B24 ZolarKeth's University on its base face, and each University once a turn.
//!  - B25 Going Long and Aluneth are no Quickdraw cards; B26 the five new costs.
//!  - B27 Pestilent Slime spreads its counters one placement each; B28 Memory Leak's Crater.
//!  - B29 Starfleet Commander; B30 Preemptive Strike; B31 Bob's Plan's Star Chart; B32 Star Chart.
//!  - B33, B34 CN Tech's and NA Tech's fallbacks; B35 Fruit Launcher's Grapes.
//!  - B36 the Resource and Faction tags.

use jackioh_cards::{CATALOG, card_def, register_all};
use jackioh_engine::testkit::*;

use super::{scenario, targets_of};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

/// Core #8 Mr. Vanilla: a 4/4 with no text, the filler of every hand, library and board here.
const VANILLA: &str = "core-008";
/// Classic #86 Genn: a 14/14 (Radiant 42/42) with no text, a wall.
const GENN: &str = "classic-086";
/// Cards in each library, so no draw of these tests fatigues.
const LIBRARY: usize = 10;

const METEOR: &str = "stinkyscape-003";
const TMACK_TRAV: &str = "stinkyscape-094";
const ZOLARKETH: &str = "stinkyscape-026";
const UNIVERSITY: &str = "meditative-097-6";
const GOING_LONG: &str = "core-084";
const ALUNETH: &str = "meditative-080";
const OTHERWORLDLY_MAGIC: &str = "stinkyscape-011";
const THE_NARCOR_PROJECT: &str = "stinkyscape-039";
const THE_SYNDICATE: &str = "stinkyscape-164";
const SWEAR_LOYALITIES: &str = "stinkyscape-163";
const MEMORY_LEAK: &str = "classicplus-034";
const PESTILENT_SLIME: &str = "classic-027";
/// Core #41 Sheepish Trap: reveals only when the opponent plays a Unit, so it lies face-down here.
const SHEEPISH_TRAP: &str = "core-041";
const STARFLEET_COMMANDER: &str = "stinkyscape-137";
const PREEMPTIVE_STRIKE: &str = "stinkyscape-176";
/// S #100 AP Rounds: a 3/1 Starship Piece with no text, whose death gives its owner a Dormant Starship.
const AP_ROUNDS: &str = "stinkyscape-100";
const WAR_DOCTRINES: [&str; 4] = [
    "stinkyscape-137-1",
    "stinkyscape-137-2",
    "stinkyscape-137-3",
    "stinkyscape-137-4",
];
const BOBS_PLAN: &str = "meditativeplus-001";
const STAR_CHART: &str = "meditativeplus-001-5";
const ALTERAN_REACH: &str = "stinkyscape-148";
const EDGE_OF_THE_UNIVERSE: &str = "stinkyscape-148-1";
const REACH_THE_SUMMIT: &str = "meditative-008";
const BASE_CAMP: &str = "meditativeplus-001-1";
const AESTHETICIZE_THE_GAME: &str = "meditative-020";
const CN_TECH: &str = "meditative-050";
const NA_TECH: &str = "meditativeplus-002";
const LUNAR_ECLIPSE: &str = "core-035";
const INFINITE_RESERVES: &str = "core-075";
const BIG_D_FENDER: &str = "core-001";
const FRUIT_LAUNCHER: &str = "stinkyscape-141";
const GRAPES: [&str; 5] = [
    "classicplus-065-1",
    "classicplus-065-2",
    "classicplus-065-3",
    "classicplus-065-4",
    "classicplus-065-5",
];

/// B21: Meteor's damage on both faces.
const METEOR_DAMAGE: i32 = 15;
/// B32: Star Chart's `nodes`, base and Radiant.
const STAR_CHART_NODES: i32 = 2;
const RADIANT_STAR_CHART_NODES: i32 = 4;
/// B36: the census of the two new tags.
const RESOURCE_CARDS: usize = 6;
const FACTION_CARDS: usize = 5;

// ---------------------------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------------------------

/// p1's main phase on the harness's mid-game turn: each side as given, a Mr. Vanilla in hand and a
/// library of them where the side names none; neither side's turn ends by itself (R82).
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    let side = |mut side: Value| {
        if side.get("library").is_none() {
            side["library"] = json!(vec![VANILLA; LIBRARY]);
        }
        if side.get("hand").is_none() {
            side["hand"] = json!([VANILLA]);
        }
        side
    };
    let mut s = scenario(json!({ "seed": seed, "p1": side(p1), "p2": side(p2) }));
    s.state_mut().players.p1.auto_end_turn = Some(false);
    s.state_mut().players.p2.auto_end_turn = Some(false);
    s
}

fn on(def: &str, radiant: bool) -> Value {
    json!({ "def": def, "radiant": radiant })
}

fn hand_of(s: &Scenario, def: &str) -> Vec<CardInstance> {
    s.hand(P1).into_iter().filter(|card| card.def_id == def).collect()
}

fn enemy_hero() -> String {
    format!("hero-{}", P2.as_str())
}

fn slot(player: PlayerId, row: Row, lane: i32) -> ZoneSlot {
    ZoneSlot { player, row, lane }
}

/// The first option of the prompt open now, which must be `player`'s.
fn answer_first(s: &mut Scenario, player: PlayerId) {
    let pending = s.state().pending.clone().expect("a prompt is open");
    assert_eq!(pending.player_id, player, "the prompt is {player:?}'s");
    let key = pending
        .options
        .first()
        .expect("the prompt offers an option")
        .key
        .clone();
    s.answer(json!(key));
}

// ---------------------------------------------------------------------------------------------
// B21 Meteor
// ---------------------------------------------------------------------------------------------

fn meteor_board(seed: &str, radiant: bool, p2_field: Value) -> Scenario {
    board(
        seed,
        json!({ "hand": [on(METEOR, radiant), VANILLA] }),
        json!({ "field": p2_field }),
    )
}

#[test]
fn b21_meteor_deals_15_to_its_target_on_both_faces() {
    for radiant in [false, true] {
        // A lone Radiant Genn (42 Health) takes the whole hit and survives it: nothing is left over.
        let mut s = meteor_board(
            "b21-fifteen",
            radiant,
            json!([{ "def": GENN, "radiant": true, "lane": 3 }]),
        );
        let genn = s.unit_at(P2, 3);
        let before = s.stats(&genn).health;
        s.play(METEOR, json!({ "targets": pick::instance(&genn) }));
        assert_eq!(
            s.stats(&genn).health,
            before - METEOR_DAMAGE,
            "Meteor (radiant {radiant}) deals {METEOR_DAMAGE}"
        );
        assert_eq!(
            s.state().players.p2.hero.health,
            HERO_HEALTH,
            "no excess, nothing to the hero"
        );
    }
}

#[test]
fn b21_radiant_meteor_tramples_its_excess_into_the_hero() {
    let mut s = meteor_board("b21-radiant-hero", true, json!([{ "def": VANILLA, "lane": 3 }]));
    let vanilla = s.unit_at(P2, 3);
    let health = s.stats(&vanilla).health;
    s.play(METEOR, json!({ "targets": pick::instance(&vanilla) }));
    s.expect_in_zone(&vanilla, "graveyard");
    assert_eq!(
        s.state().players.p2.hero.health,
        HERO_HEALTH - (METEOR_DAMAGE - health),
        "with no neighbor, the Radiant face's excess past the target goes to its hero"
    );
}

#[test]
fn b21_base_meteor_never_reaches_the_hero() {
    let mut s = meteor_board("b21-base-hero", false, json!([{ "def": VANILLA, "lane": 3 }]));
    let vanilla = s.unit_at(P2, 3);
    s.play(METEOR, json!({ "targets": pick::instance(&vanilla) }));
    s.expect_in_zone(&vanilla, "graveyard");
    assert_eq!(
        s.state().players.p2.hero.health,
        HERO_HEALTH,
        "the base face tramples onto adjacent Units only"
    );
}

#[test]
fn b21_radiant_meteor_stops_at_a_neighbour_that_takes_the_whole_excess() {
    let mut s = meteor_board(
        "b21-neighbor",
        true,
        json!([
            { "def": GENN, "lane": 2 },
            { "def": VANILLA, "lane": 3 },
            { "def": GENN, "lane": 4 },
        ]),
    );
    let left = s.unit_at(P2, 2);
    let target = s.unit_at(P2, 3);
    let right = s.unit_at(P2, 4);
    let health = s.stats(&target).health;
    s.play(METEOR, json!({ "targets": pick::instance(&target) }));
    s.expect_in_zone(&target, "graveyard");
    s.expect_in_zone(&left, "field").expect_in_zone(&right, "field");
    assert_eq!(
        s.card(&left).damage + s.card(&right).damage,
        METEOR_DAMAGE - health,
        "the excess goes onto a neighbor, which takes it whole"
    );
    assert_eq!(
        s.state().players.p2.hero.health,
        HERO_HEALTH,
        "so none reaches the hero"
    );
}

// ---------------------------------------------------------------------------------------------
// B22 Tmack Trav
// ---------------------------------------------------------------------------------------------

/// p1 holds Tmack Trav; p2 has a Mr. Vanilla in lanes 1 and 2.
fn tmack_board(seed: &str, radiant: bool) -> Scenario {
    board(
        seed,
        json!({ "hand": [on(TMACK_TRAV, radiant), VANILLA] }),
        json!({ "field": [VANILLA, VANILLA] }),
    )
}

/// The targets `legal_actions` offers `attacker` now.
fn attack_targets(s: &Scenario, attacker: &CardInstance) -> Vec<String> {
    legal_actions(s.state(), P1)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::Attack {
                attacker_id,
                target_id,
            } if attacker_id == attacker.id => Some(target_id),
            _ => None,
        })
        .collect()
}

/// Tmack Trav played from p1's hand with Rush, then sent into p2's lane-1 Mr. Vanilla, which dies.
fn tmack_kills_on_its_rush_turn(s: &mut Scenario) -> CardInstance {
    s.play(TMACK_TRAV, json!({}));
    let tmack = s.card(TMACK_TRAV).clone();
    let first = s.unit_at(P2, 1);
    s.attack(&tmack, &first);
    s.expect_in_zone(&first, "graveyard");
    tmack
}

#[test]
fn b22_tmack_after_a_kill_on_its_rush_turn_may_attack_another_enemy_unit() {
    for radiant in [false, true] {
        let mut s = tmack_board("b22-again", radiant);
        let second = s.unit_at(P2, 2);
        let tmack = tmack_kills_on_its_rush_turn(&mut s);
        assert!(
            attack_targets(&s, &tmack).contains(&second.id),
            "radiant {radiant}: the extra attack is offered at an enemy Unit"
        );
        s.attack(&tmack, &second);
        s.expect_in_zone(&second, "graveyard");
    }
}

#[test]
fn b22_tmack_after_a_kill_on_its_rush_turn_is_not_offered_the_hero_and_is_refused_it() {
    for radiant in [false, true] {
        let mut s = tmack_board("b22-no-hero", radiant);
        let tmack = tmack_kills_on_its_rush_turn(&mut s);
        assert!(
            !attack_targets(&s, &tmack).contains(&enemy_hero()),
            "radiant {radiant}: the enemy hero is not offered"
        );
        s.expect_refused(|s| s.attack(&tmack, "hero"));
        assert_eq!(s.state().players.p2.hero.health, HERO_HEALTH);
    }
}

#[test]
fn b22_tmack_on_a_later_turn_may_send_its_extra_attack_at_the_hero() {
    for radiant in [false, true] {
        let mut s = tmack_board("b22-later", radiant);
        s.play(TMACK_TRAV, json!({}));
        let tmack = s.card(TMACK_TRAV).clone();
        s.end_turn().end_turn();
        assert_eq!(s.state().active, P1, "back to p1's next turn");
        let first = s.unit_at(P2, 1);
        s.attack(&tmack, &first);
        s.expect_in_zone(&first, "graveyard");
        assert!(
            attack_targets(&s, &tmack).contains(&enemy_hero()),
            "radiant {radiant}: on a later turn the extra attack is offered at the hero"
        );
        let attack = s.stats(&tmack).attack;
        s.attack(&tmack, "hero");
        assert_eq!(s.state().players.p2.hero.health, HERO_HEALTH - attack);
    }
}

// ---------------------------------------------------------------------------------------------
// B23, B24 ZolarKeth and its Universities
// ---------------------------------------------------------------------------------------------

fn activations_listed(s: &Scenario, card: &CardInstance) -> usize {
    legal_actions(s.state(), P1)
        .iter()
        .filter(|action| {
            matches!(action, ActionBody::Activate { instance_id, .. } | ActionBody::ActivatePower { instance_id, .. }
                if *instance_id == card.id)
        })
        .count()
}

/// R767000: `run` folding under the build's catalog with ZolarKeth's script kept for v0.5.9f, the one
/// #2521's B23 and B24 test: #2657 gave it a Cry of two modes and no Universities to run.
fn under_v0_5_9f<T>(run: impl FnOnce() -> T) -> T {
    register_all();
    let (_, _, kept) = jackioh_cards::KEPT_SCRIPTS
        .iter()
        .find(|(id, version, _)| *id == ZOLARKETH && *version == "v0.5.9f")
        .expect("ZolarKeth's v0.5.9f script is kept");
    let shipped: &'static jackioh_engine::ShippedCatalog =
        Box::leak(Box::new(jackioh_engine::ShippedCatalog::new(
            "kept",
            CATALOG.clone(),
            IndexMap::from([(ZOLARKETH.to_string(), kept())]),
        )));
    jackioh_engine::with_shipped(Some(shipped), run)
}

fn zolarketh_played(seed: &str, radiant: bool) -> (Scenario, CardInstance) {
    let mut s = board(
        seed,
        json!({ "hand": [on(ZOLARKETH, radiant), VANILLA] }),
        json!({}),
    );
    s.play(ZOLARKETH, json!({}));
    let university = s.card(UNIVERSITY).clone();
    s.expect_in_zone(&university, "field");
    (s, university)
}

#[test]
fn b23_zolarketh_summons_a_university_on_its_base_face_from_both_faces() {
    under_v0_5_9f(|| {
        for radiant in [false, true] {
            let (s, university) = zolarketh_played("b23-face", radiant);
            assert_eq!(university.controller, P1);
            assert!(
                !s.card(&university).radiant,
                "ZolarKeth (radiant {radiant}) summons a base-face University"
            );
        }
    });
}

#[test]
fn b23_the_summoned_university_is_refused_its_own_activate_now_and_on_a_later_turn() {
    under_v0_5_9f(|| {
        for radiant in [false, true] {
            let (mut s, university) = zolarketh_played("b23-refused", radiant);
            assert_eq!(
                activations_listed(&s, &university),
                0,
                "none of its abilities is listed"
            );
            s.expect_refused(|s| s.activate(&university, json!({})));
            s.end_turn().end_turn();
            assert_eq!(s.state().active, P1);
            assert_eq!(
                activations_listed(&s, &university),
                0,
                "still none on a later turn"
            );
            s.expect_refused(|s| s.activate(&university, json!({})));
        }
    });
}

/// p1's ZolarKeths (on the faces given) from lane 1 on, then two Universities, both already on the
/// field (so each keeps its own Activate).
struct Campus {
    s: Scenario,
    zolars: Vec<CardInstance>,
    first: CardInstance,
    second: CardInstance,
}

fn campus(seed: &str, zolars: &[bool]) -> Campus {
    let mut field: Vec<Value> = zolars.iter().map(|radiant| on(ZOLARKETH, *radiant)).collect();
    field.push(json!(UNIVERSITY));
    field.push(json!(UNIVERSITY));
    let s = board(seed, json!({ "field": field }), json!({}));
    let lanes = zolars.len() as i32;
    let zolars: Vec<CardInstance> = (1..=lanes).map(|lane| s.unit_at(P1, lane)).collect();
    let first = s.unit_at(P1, lanes + 1);
    let second = s.unit_at(P1, lanes + 2);
    assert_eq!(
        (first.def_id.as_str(), second.def_id.as_str()),
        (UNIVERSITY, UNIVERSITY)
    );
    Campus {
        s,
        zolars,
        first,
        second,
    }
}

/// ZolarKeth's listed Activate that runs the Universities: on the base face the mode that is not
/// "summon a random Starship Piece"; on the Radiant face its one ability.
fn universities_action(s: &Scenario, zolar: &CardInstance) -> ActionBody {
    legal_actions(s.state(), P1)
        .into_iter()
        .find(|action| match action {
            ActionBody::Activate {
                instance_id, modes, ..
            } => {
                *instance_id == zolar.id
                    && (zolar.radiant || modes.as_ref().is_some_and(|modes| !modes.is_empty()))
                    && !modes.iter().flatten().any(|mode| mode == "piece")
            }
            _ => false,
        })
        .unwrap_or_else(|| {
            panic!(
                "legal_actions lists no `Activate your Universities` on {}",
                zolar.id
            )
        })
}

/// How many Buffs (`upgraded`) `events` gave `card`.
fn buffs_on(events: &[GameEvent], card: &CardInstance) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, GameEvent::Upgraded { instance_id, .. } if *instance_id == card.id))
        .count()
}

/// Which of the two Universities ran in `events`: a University Buffs each of its OTHER permanents, so
/// the first ran exactly when the second was Buffed, and the other way round.
fn ran(events: &[GameEvent], first: &CardInstance, second: &CardInstance) -> (bool, bool) {
    (buffs_on(events, second) > 0, buffs_on(events, first) > 0)
}

#[test]
fn b24_zolarketh_activates_each_university_with_a_use_left() {
    under_v0_5_9f(|| {
        for radiant in [false, true] {
            let Campus {
                mut s,
                zolars,
                first,
                second,
            } = campus("b24-both", &[radiant]);
            let action = universities_action(&s, &zolars[0]);
            s.act(action, P1, "ZolarKeth activates the Universities");
            assert_eq!(
                ran(s.last_events(), &first, &second),
                (true, true),
                "radiant {radiant}: both Universities ran"
            );
        }
    });
}

#[test]
fn b24_a_university_zolarketh_ran_has_spent_its_use_and_is_refused_its_own_activate() {
    under_v0_5_9f(|| {
        for radiant in [false, true] {
            let Campus {
                mut s,
                zolars,
                first,
                second,
            } = campus("b24-spent", &[radiant]);
            let action = universities_action(&s, &zolars[0]);
            s.act(action, P1, "ZolarKeth activates the Universities");
            for university in [&first, &second] {
                assert_eq!(
                    activations_listed(&s, university),
                    0,
                    "radiant {radiant}: its use is spent"
                );
                s.expect_refused(|s| s.activate(university, json!({})));
            }
        }
    });
}

#[test]
fn b24_a_university_that_activated_itself_this_turn_is_skipped() {
    under_v0_5_9f(|| {
        let Campus {
            mut s,
            zolars,
            first,
            second,
        } = campus("b24-skipped", &[false]);
        s.activate(&first, json!({}));
        assert_eq!(
            ran(s.last_events(), &first, &second),
            (true, false),
            "the first activated itself"
        );
        let action = universities_action(&s, &zolars[0]);
        s.act(action, P1, "ZolarKeth activates the Universities");
        assert_eq!(
            ran(s.last_events(), &first, &second),
            (false, true),
            "ZolarKeth runs only the University with a use left"
        );
        s.expect_refused(|s| s.activate(&second, json!({})));
    });
}

#[test]
fn b24_a_second_zolarketh_skips_the_universities_the_first_one_ran() {
    under_v0_5_9f(|| {
        let Campus {
            mut s,
            zolars,
            first,
            second,
        } = campus("b24-second", &[false, false]);
        // The second ZolarKeth's ability as the client would send it, read while it is listed.
        let later = universities_action(&s, &zolars[1]);
        let action = universities_action(&s, &zolars[0]);
        s.act(action, P1, "the first ZolarKeth activates the Universities");
        assert_eq!(ran(s.last_events(), &first, &second), (true, true));
        s.act(later, P1, "the second ZolarKeth activates the Universities");
        assert_eq!(
            ran(s.last_events(), &first, &second),
            (false, false),
            "no University runs twice in a turn"
        );
    });
}

#[test]
fn b24_a_new_turn_gives_each_university_its_use_back() {
    under_v0_5_9f(|| {
        let Campus {
            mut s,
            zolars,
            first,
            second,
        } = campus("b24-new-turn", &[false]);
        let action = universities_action(&s, &zolars[0]);
        s.act(action, P1, "ZolarKeth activates the Universities");
        s.expect_refused(|s| s.activate(&first, json!({})));
        s.end_turn().end_turn();
        assert_eq!(s.state().active, P1);
        s.activate(&first, json!({}));
        assert_eq!(
            ran(s.last_events(), &first, &second),
            (true, false),
            "its use came back with the turn"
        );
    });
}

// ---------------------------------------------------------------------------------------------
// B25 Going Long and Aluneth are no Quickdraw cards
// ---------------------------------------------------------------------------------------------

/// 24 plain Core cards, none Quickdraw and none cast on draw: with one more card, a deck.
const PLAIN_DECK: [&str; 24] = [
    "core-002", "core-005", "core-006", "core-008", "core-011", "core-012", "core-013", "core-015",
    "core-016", "core-019", "core-020", "core-025", "core-026", "core-032", "core-036", "core-043",
    "core-044", "core-053", "core-055", "core-001", "core-003", "core-004", "core-009", "core-010",
];

/// Both seats' decks the plain cards and `card`, through setup's opening deal.
fn opening_deal(seed: &str, card: &str) -> GameState {
    register_all();
    let mut deck: Vec<String> = PLAIN_DECK.iter().map(|id| (*id).to_string()).collect();
    deck.push(card.to_string());
    assert_eq!(deck.len(), DECK_SIZE as usize);
    let created = create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks: (deck.clone(), deck),
        ..CreateGameOptions::default()
    });
    begin_game(&created).state
}

fn not_dealt_by_setup(card: &str) {
    assert!(
        !card_def(card).tags.contains(&Tag::Quickdraw),
        "{card} carries no Quickdraw tag"
    );
    // A Quickdraw card is dealt into every opening hand; this one stays in the library on some seed.
    let seed = seed_where(&format!("b25-{card}"), |seed| {
        let state = opening_deal(seed, card);
        !state.players.p1.hand.iter().any(|dealt| dealt.def_id == card)
    });
    let state = opening_deal(&seed, card);
    assert!(state.players.p1.library.iter().any(|left| left.def_id == card));
}

#[test]
fn b25_going_long_is_not_dealt_into_the_opening_hand() {
    not_dealt_by_setup(GOING_LONG);
}

#[test]
fn b25_aluneth_is_not_dealt_into_the_opening_hand() {
    not_dealt_by_setup(ALUNETH);
}

// ---------------------------------------------------------------------------------------------
// B26 the costs
// ---------------------------------------------------------------------------------------------

/// Each card, the face its play is tested on (Memory Leak's Radiant face asks no "Choose one."), and
/// its new cost.
const COSTS: [(&str, bool, i32); 5] = [
    (OTHERWORLDLY_MAGIC, false, 2),
    (THE_NARCOR_PROJECT, false, 3),
    // #2657 took The Syndicate to (0).
    (THE_SYNDICATE, false, 0),
    (SWEAR_LOYALITIES, false, 0),
    (MEMORY_LEAK, true, 2),
];

fn priced(card: &str, radiant: bool, mana: i32) -> Scenario {
    board(
        &format!("b26-{card}"),
        json!({ "hand": [on(card, radiant), VANILLA], "mana": mana }),
        json!({}),
    )
}

#[test]
fn b26_the_catalog_prints_the_new_costs() {
    for (card, _, cost) in COSTS {
        assert_eq!(card_def(card).cost, CardCost::Fixed(cost), "{card}");
    }
}

/// The plays `legal_actions` lists for p1's copy of `card` in hand.
fn plays_listed(s: &Scenario, card: &str) -> Vec<ActionBody> {
    let held = s.card(card).id.clone();
    legal_actions(s.state(), P1)
        .into_iter()
        .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == held))
        .collect()
}

#[test]
fn b26_one_mana_short_of_the_new_cost_is_refused() {
    for (card, radiant, cost) in COSTS.into_iter().filter(|(_, _, cost)| *cost > 0) {
        let mut s = priced(card, radiant, cost - 1);
        assert!(
            plays_listed(&s, card).is_empty(),
            "{card} is not offered at {} mana",
            cost - 1
        );
        s.expect_refused_with(
            |s| s.play(card, json!({})),
            &format!("costs {cost}, more than your mana"),
        );
        assert_eq!(hand_of(&s, card).len(), 1, "{card} stays in hand");
    }
}

#[test]
fn b26_exactly_the_new_cost_plays_it() {
    for (card, radiant, cost) in COSTS {
        let mut s = priced(card, radiant, cost);
        let play = plays_listed(&s, card)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("{card} is offered at {cost} mana"));
        s.act(play, P1, "play it for exactly its cost");
        assert!(hand_of(&s, card).is_empty(), "{card} was played");
        assert_eq!(s.state().players.p1.mana.current, 0, "{card} cost all {cost}");
    }
}

// ---------------------------------------------------------------------------------------------
// B27 Pestilent Slime
// ---------------------------------------------------------------------------------------------

/// p1's Pestilent Slime in lane 1 holding `plague` counters, then `field` and `backrow`; p2's Mr.
/// Vanilla in lane 1, which the Slime attacks and dies to.
fn slime_board(seed: &str, radiant: bool, plague: i32, field: Value, backrow: Value) -> Scenario {
    let mut slime = on(PESTILENT_SLIME, radiant);
    if plague > 0 {
        slime["counters"] = json!({ "plague": plague });
    }
    let mut units = vec![slime];
    units.extend(field.as_array().cloned().unwrap_or_default());
    board(
        seed,
        json!({ "field": units, "backrow": backrow }),
        json!({ "field": [VANILLA] }),
    )
}

fn slime_dies(s: &mut Scenario) {
    let slime = s.unit_at(P1, 1);
    assert_eq!(slime.def_id, PESTILENT_SLIME);
    let wall = s.unit_at(P2, 1);
    s.attack(&slime, &wall);
    s.expect_in_zone(&slime, "graveyard");
}

/// Every permanent `player` has on the field: the units on top of their zones, then the backrow.
fn permanents(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
    (1..=UNIT_ZONES)
        .filter_map(|lane| s.unit(player, lane))
        .chain((1..=BACKROW_ZONES).filter_map(|lane| s.backrow(player, lane)))
        .collect()
}

fn plague_of(card: &CardInstance) -> i32 {
    card.counters.plague.unwrap_or(0)
}

fn plague_on_field(s: &Scenario, player: PlayerId) -> i32 {
    permanents(s, player).iter().map(plague_of).sum()
}

/// The Plague Counter placements in `events`, each as the number it placed.
fn placements(events: &[GameEvent]) -> Vec<i32> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::CounterChanged {
                counter,
                placed: Some(placed),
                ..
            } if *counter == CounterKind::Plague => Some(*placed),
            _ => None,
        })
        .collect()
}

#[test]
fn b27_each_counter_is_placed_once_on_a_permanent_of_its_controller() {
    for radiant in [false, true] {
        let mut s = slime_board(
            "b27-spread",
            radiant,
            3,
            json!([VANILLA]),
            json!([{ "def": SHEEPISH_TRAP, "faceUp": false }]),
        );
        slime_dies(&mut s);
        assert_eq!(
            plague_on_field(&s, P1),
            3,
            "radiant {radiant}: all three counters are on p1's side"
        );
        assert_eq!(
            placements(s.last_events()),
            vec![1, 1, 1],
            "one placement for each counter"
        );
    }
}

#[test]
fn b27_the_opponents_permanents_never_receive_a_counter() {
    let mut s = slime_board("b27-enemy", false, 3, json!([VANILLA]), json!([]));
    slime_dies(&mut s);
    assert_eq!(plague_on_field(&s, P2), 0);
    assert_eq!(plague_on_field(&s, P1), 3);
}

#[test]
fn b27_a_face_down_backrow_card_takes_them_when_it_is_the_only_permanent() {
    let mut s = slime_board(
        "b27-face-down",
        false,
        3,
        json!([]),
        json!([{ "def": SHEEPISH_TRAP, "faceUp": false }]),
    );
    let trap = s.backrow(P1, 1).expect("the Trap lies in lane 1");
    slime_dies(&mut s);
    assert_eq!(plague_of(s.card(&trap)), 3);
}

#[test]
fn b27_a_face_up_field_spell_takes_them_when_it_is_the_only_permanent() {
    let mut s = slime_board(
        "b27-face-up",
        true,
        2,
        json!([]),
        json!([{ "def": GOING_LONG, "faceUp": true }]),
    );
    let field_spell = s.backrow(P1, 1).expect("Going Long stands in lane 1");
    slime_dies(&mut s);
    assert_eq!(plague_of(s.card(&field_spell)), 2);
}

#[test]
fn b27_each_placement_lands_on_a_random_permanent() {
    let spread = |seed: &str| {
        let mut s = slime_board(
            seed,
            false,
            3,
            json!([VANILLA]),
            json!([{ "def": SHEEPISH_TRAP, "faceUp": false }]),
        );
        let vanilla = s.unit_at(P1, 2);
        let trap = s.backrow(P1, 1).expect("the Trap lies in lane 1");
        slime_dies(&mut s);
        (plague_of(s.card(&vanilla)), plague_of(s.card(&trap)))
    };
    // Some seed splits the three counters between the two permanents: no one draw takes them all.
    let seed = seed_where("b27-split", |seed| {
        let (unit, trap) = spread(seed);
        unit > 0 && trap > 0
    });
    let (unit, trap) = spread(&seed);
    assert_eq!(unit + trap, 3);
}

#[test]
fn b27_with_no_permanent_left_the_counters_are_lost() {
    let mut s = slime_board("b27-lost", false, 3, json!([]), json!([]));
    slime_dies(&mut s);
    assert!(permanents(&s, P1).is_empty(), "p1 has nothing on the field");
    assert_eq!(
        placements(s.last_events()),
        Vec::<i32>::new(),
        "nothing is placed"
    );
    assert_eq!(plague_on_field(&s, P2), 0, "and nothing goes to the opponent");
}

#[test]
fn b27_with_no_counters_nothing_is_placed() {
    for radiant in [false, true] {
        let mut s = slime_board(
            "b27-none",
            radiant,
            0,
            json!([VANILLA]),
            json!([{ "def": SHEEPISH_TRAP, "faceUp": false }]),
        );
        slime_dies(&mut s);
        assert_eq!(placements(s.last_events()), Vec::<i32>::new());
        assert_eq!(plague_on_field(&s, P1), 0);
    }
}

// ---------------------------------------------------------------------------------------------
// B28 Memory Leak's Crater
// ---------------------------------------------------------------------------------------------

/// Memory Leak from p1's hand into backrow lane 2, by the play `legal_actions` lists for that zone (the
/// base face's "Choose one." included), any prompt the play opens answered with its first option.
fn memory_leak_into_lane_2(radiant: bool) -> Scenario {
    let mut s = board(
        &format!("b28-{radiant}"),
        json!({ "hand": [on(MEMORY_LEAK, radiant), VANILLA] }),
        json!({}),
    );
    let leak = s.card(MEMORY_LEAK).clone();
    let zone = ZoneChoice {
        row: Row::Backrow,
        lane: 2,
    };
    assert!(
        !is_locked(s.state(), slot(P1, Row::Backrow, 2)),
        "not Locked before"
    );
    let play = legal_actions(s.state(), P1)
        .into_iter()
        .find(|action| {
            matches!(action, ActionBody::Play { instance_id, zone: Some(at), .. }
                if *instance_id == leak.id && *at == zone)
        })
        .expect("a play of Memory Leak into backrow lane 2 is listed");
    s.act(play, P1, "play Memory Leak into backrow lane 2");
    if s.state().pending.is_some() {
        answer_first(&mut s, P1);
    }
    assert_eq!(
        s.backrow(P1, 2).map(|card| card.def_id),
        Some(MEMORY_LEAK.to_string()),
        "Memory Leak stands in lane 2"
    );
    s
}

#[test]
fn b28_memory_leak_locks_its_backrow_zone_as_it_enters() {
    assert!(
        card_def(MEMORY_LEAK)
            .base
            .keywords
            .iter()
            .any(|keyword| keyword.kind() == KeywordKind::Crater),
        "Memory Leak prints Crater"
    );
    for radiant in [false, true] {
        let s = memory_leak_into_lane_2(radiant);
        assert!(
            is_locked(s.state(), slot(P1, Row::Backrow, 2)),
            "radiant {radiant}: its zone is Locked"
        );
    }
}

#[test]
fn b28_memory_leak_locks_no_other_zone() {
    for radiant in [false, true] {
        let s = memory_leak_into_lane_2(radiant);
        for lane in (1..=BACKROW_ZONES).filter(|lane| *lane != 2) {
            assert!(
                !is_locked(s.state(), slot(P1, Row::Backrow, lane)),
                "p1's backrow lane {lane}"
            );
        }
        for lane in 1..=UNIT_ZONES {
            assert!(
                !is_locked(s.state(), slot(P1, Row::Units, lane)),
                "p1's unit lane {lane}"
            );
            assert!(
                !is_locked(s.state(), slot(P2, Row::Units, lane)),
                "p2's unit lane {lane}"
            );
        }
        for lane in 1..=BACKROW_ZONES {
            assert!(
                !is_locked(s.state(), slot(P2, Row::Backrow, lane)),
                "p2's backrow lane {lane}"
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// B29 Starfleet Commander, B30 Preemptive Strike
// ---------------------------------------------------------------------------------------------

/// p1's AP Rounds attacks p2's Genn and dies, so p1 has a Dormant Starship; p1 holds `hand`.
fn with_dormant_starship(seed: &str, hand: Value) -> (Scenario, String) {
    let mut s = board(
        seed,
        json!({ "hand": hand, "field": [AP_ROUNDS] }),
        json!({ "field": [GENN] }),
    );
    let piece = s.unit_at(P1, 1);
    let wall = s.unit_at(P2, 1);
    s.attack(&piece, &wall);
    let ship = dormant_starship_of(s.state(), P1)
        .expect("setup: the dead piece gave p1 a Dormant Starship")
        .id
        .clone();
    (s, ship)
}

fn launches_of(s: &Scenario, player: PlayerId) -> Vec<String> {
    s.events()
        .iter()
        .filter_map(|event| match event {
            GameEvent::StarshipLaunched {
                player: who,
                instance_id,
                ..
            } if *who == player => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

/// The War Doctrines that resolved for p1, each with the face it resolved on.
fn doctrines_cast(s: &Scenario) -> Vec<(String, bool)> {
    s.events()
        .iter()
        .filter_map(|event| match event {
            GameEvent::CardResolved {
                player: PlayerId::P1,
                def_id,
                radiant,
                ..
            } if WAR_DOCTRINES.contains(&def_id.as_str()) => Some((def_id.clone(), radiant.unwrap_or(false))),
            _ => None,
        })
        .collect()
}

/// `card` (a Unit or a Spell) played with p1's Dormant Starship: the Starship launches, then a War
/// Doctrine is Discovered and cast, on the card's face.
fn launches_then_casts_a_war_doctrine(card: &str, seed: &str) {
    for radiant in [false, true] {
        let (mut s, ship) = with_dormant_starship(seed, json!([on(card, radiant), VANILLA]));
        s.play(card, json!({}));
        assert_eq!(
            launches_of(&s, P1),
            vec![ship.clone()],
            "radiant {radiant}: the Starship launched"
        );
        let launched_at = s
            .events()
            .iter()
            .position(|event| matches!(event, GameEvent::StarshipLaunched { .. }))
            .expect("a launch");
        let asked_at = s
            .events()
            .iter()
            .position(|event| {
                matches!(
                    event,
                    GameEvent::PromptOpened {
                        player: PlayerId::P1,
                        ..
                    }
                )
            })
            .expect("the Discover asks p1");
        assert!(launched_at < asked_at, "the launch comes before the Discover");
        answer_first(&mut s, P1);
        let cast = doctrines_cast(&s);
        assert_eq!(cast.len(), 1, "one War Doctrine is cast: {cast:?}");
        assert_eq!(cast[0].1, radiant, "on the face of the card that cast it");
        assert!(
            dormant_starship_of(s.state(), P1).is_none(),
            "no Dormant Starship is left"
        );
    }
}

/// `card` played with no Dormant Starship on either side: nothing launches and nothing is asked.
fn does_nothing_without_a_dormant_starship(card: &str, seed: &str) {
    for radiant in [false, true] {
        let mut s = board(seed, json!({ "hand": [on(card, radiant), VANILLA] }), json!({}));
        s.play(card, json!({}));
        assert!(s.state().pending.is_none(), "radiant {radiant}: no Discover");
        assert!(launches_of(&s, P1).is_empty(), "nothing launched");
        assert!(doctrines_cast(&s).is_empty(), "no War Doctrine cast");
    }
}

#[test]
fn b29_starfleet_commander_is_a_legendary_human_unit() {
    let def = card_def(STARFLEET_COMMANDER);
    assert_eq!(def.name, "Starfleet Commander");
    assert_eq!(def.type_, CardType::Unit);
    assert_eq!(def.rarity, Rarity::Legendary);
    assert!(def.tags.contains(&Tag::Human));
}

#[test]
fn b29_starfleet_commanders_cry_launches_the_dormant_starship_and_casts_a_war_doctrine() {
    launches_then_casts_a_war_doctrine(STARFLEET_COMMANDER, "b29-cry");
}

#[test]
fn b29_starfleet_commanders_cry_does_nothing_without_a_dormant_starship() {
    does_nothing_without_a_dormant_starship(STARFLEET_COMMANDER, "b29-nothing");
}

#[test]
fn b30_preemptive_strike_is_a_legendary_spell_costing_2() {
    let def = card_def(PREEMPTIVE_STRIKE);
    assert_eq!(def.type_, CardType::Spell);
    assert_eq!(def.rarity, Rarity::Legendary);
    assert_eq!(def.cost, CardCost::Fixed(2));
    let mut s = board(
        "b30-price",
        json!({ "hand": [PREEMPTIVE_STRIKE, VANILLA], "mana": 1 }),
        json!({}),
    );
    s.expect_refused_with(
        |s| s.play(PREEMPTIVE_STRIKE, json!({})),
        "costs 2, more than your mana",
    );
}

#[test]
fn b30_preemptive_strike_launches_the_dormant_starship_and_casts_a_war_doctrine() {
    launches_then_casts_a_war_doctrine(PREEMPTIVE_STRIKE, "b30-cast");
}

#[test]
fn b30_preemptive_strike_does_nothing_without_a_dormant_starship() {
    does_nothing_without_a_dormant_starship(PREEMPTIVE_STRIKE, "b30-nothing");
}

#[test]
fn b30_preemptive_strike_never_launches_the_opponents_dormant_starship() {
    let mut s = board(
        "b30-theirs",
        json!({ "hand": [PREEMPTIVE_STRIKE, VANILLA], "field": [GENN] }),
        json!({ "field": [AP_ROUNDS] }),
    );
    let wall = s.unit_at(P1, 1);
    let piece = s.unit_at(P2, 1);
    s.attack(&wall, &piece);
    let theirs = dormant_starship_of(s.state(), P2)
        .expect("setup: p2's dead piece gave p2 a Dormant Starship")
        .id
        .clone();
    s.play(PREEMPTIVE_STRIKE, json!({}));
    assert!(s.state().pending.is_none(), "no Discover");
    assert!(
        launches_of(&s, P1).is_empty() && launches_of(&s, P2).is_empty(),
        "nothing launched"
    );
    assert_eq!(
        dormant_starship_of(s.state(), P2).map(|ship| ship.id.clone()),
        Some(theirs),
        "p2's Starship is still Dormant"
    );
}

// ---------------------------------------------------------------------------------------------
// B31 Bob's Plan
// ---------------------------------------------------------------------------------------------

fn bobs_plan(seed: &str, radiant: bool) -> Scenario {
    let mut s = board(
        seed,
        json!({ "hand": [on(BOBS_PLAN, radiant), VANILLA] }),
        json!({}),
    );
    s.play(BOBS_PLAN, json!({}));
    s
}

#[test]
fn b31_bobs_plan_adds_a_star_chart_with_alteran_reach() {
    for radiant in [false, true] {
        let seed = seed_where(&format!("b31-alteran-{radiant}"), |seed| {
            !hand_of(&bobs_plan(seed, radiant), ALTERAN_REACH).is_empty()
        });
        let s = bobs_plan(&seed, radiant);
        let charts = hand_of(&s, STAR_CHART);
        assert_eq!(charts.len(), 1, "radiant {radiant}: one Star Chart");
        assert_eq!(
            charts[0].radiant, radiant,
            "a Radiant Star Chart from the Radiant Bob's Plan"
        );
        assert!(hand_of(&s, BASE_CAMP).is_empty(), "no Base Camp");
    }
}

#[test]
fn b31_reach_the_summit_keeps_its_base_camp_and_gets_no_star_chart() {
    let seed = seed_where("b31-summit", |seed| {
        !hand_of(&bobs_plan(seed, false), REACH_THE_SUMMIT).is_empty()
    });
    let s = bobs_plan(&seed, false);
    assert_eq!(hand_of(&s, BASE_CAMP).len(), 1);
    assert!(hand_of(&s, STAR_CHART).is_empty());
}

#[test]
fn b31_aestheticize_the_game_keeps_its_discover_and_gets_no_star_chart() {
    // The one plan that asks: neither of the other two Wincons came, and a Discover is open.
    let discovering = |s: &Scenario| {
        hand_of(s, ALTERAN_REACH).is_empty()
            && hand_of(s, REACH_THE_SUMMIT).is_empty()
            && s.state().pending.is_some()
    };
    let seed = seed_where("b31-aestheticize", |seed| discovering(&bobs_plan(seed, false)));
    let mut s = bobs_plan(&seed, false);
    answer_first(&mut s, P1);
    assert_eq!(hand_of(&s, AESTHETICIZE_THE_GAME).len(), 1);
    assert!(hand_of(&s, STAR_CHART).is_empty());
}

// ---------------------------------------------------------------------------------------------
// B32 Star Chart
// ---------------------------------------------------------------------------------------------

/// p1 holds a Star Chart and has `mana`; p2's hero at 100, so no node's damage ends the game.
fn chart_board(seed: &str, radiant: bool, mana: i32) -> Scenario {
    board(
        seed,
        json!({ "hand": [on(STAR_CHART, radiant), VANILLA], "mana": mana }),
        json!({ "health": 100 }),
    )
}

/// 19 Heals, then the edge.
fn heal_path() -> Vec<StarscapeReward> {
    let mut nodes = vec![StarscapeReward::Heal; (STARSCAPE_EDGE_TRAVELS - 1) as usize];
    nodes.push(StarscapeReward::Edge);
    nodes
}

fn set_map(s: &mut Scenario, at: i32) {
    s.state_mut().players.p1.starscape = Some(StarscapeMap {
        nodes: heal_path(),
        at,
        travelled_on: 0,
    });
}

fn at(s: &Scenario) -> i32 {
    s.state().players.p1.starscape.as_ref().map_or(0, |map| map.at)
}

fn traveled(events: &[GameEvent]) -> Vec<(i32, StarscapeReward)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::StarscapeTravelled {
                player: PlayerId::P1,
                node,
                reward,
            } => Some((*node, *reward)),
            _ => None,
        })
        .collect()
}

#[test]
fn b32_star_chart_is_a_0_cost_ai_token_spell() {
    let def = card_def(STAR_CHART);
    assert_eq!(def.type_, CardType::Spell);
    assert!(def.token);
    assert!(def.tags.contains(&Tag::Ai) && def.tags.contains(&Tag::Token));
    assert_eq!(def.cost, CardCost::Fixed(0));
}

#[test]
fn b32_star_chart_travels_its_nodes_along_the_map() {
    for (radiant, nodes) in [(false, STAR_CHART_NODES), (true, RADIANT_STAR_CHART_NODES)] {
        let mut s = chart_board("b32-travel", radiant, MAX_MANA);
        set_map(&mut s, 0);
        s.play(STAR_CHART, json!({}));
        assert_eq!(at(&s), nodes, "radiant {radiant}");
        assert_eq!(
            traveled(s.last_events()),
            (1..=nodes)
                .map(|node| (node, StarscapeReward::Heal))
                .collect::<Vec<_>>(),
            "one travel a node, each giving its reward"
        );
    }
}

#[test]
fn b32_star_chart_charts_the_map_when_its_caster_has_none() {
    let mut s = chart_board("b32-chart", false, MAX_MANA);
    assert_eq!(s.state().players.p1.starscape, None);
    s.play(STAR_CHART, json!({}));
    let map = s
        .state()
        .players
        .p1
        .starscape
        .clone()
        .expect("the first travel charts a map");
    assert_eq!(map.nodes.len() as i32, STARSCAPE_EDGE_TRAVELS);
    assert_eq!(map.at, STAR_CHART_NODES);
    assert_eq!(traveled(s.last_events()).len() as i32, STAR_CHART_NODES);
}

#[test]
fn b32_star_chart_stops_at_the_edge_which_adds_edge_of_the_universe() {
    // One node short of the edge: the base face's second travel is not taken.
    let mut s = chart_board("b32-edge", false, MAX_MANA);
    set_map(&mut s, STARSCAPE_EDGE_TRAVELS - 1);
    s.play(STAR_CHART, json!({}));
    assert_eq!(at(&s), STARSCAPE_EDGE_TRAVELS);
    assert_eq!(
        traveled(s.last_events()),
        vec![(STARSCAPE_EDGE_TRAVELS, StarscapeReward::Edge)]
    );
    assert_eq!(
        hand_of(&s, EDGE_OF_THE_UNIVERSE).len(),
        1,
        "the edge adds Edge of the Universe"
    );

    // Two short: the Radiant face travels two of its four nodes.
    let mut s = chart_board("b32-edge-radiant", true, MAX_MANA);
    set_map(&mut s, STARSCAPE_EDGE_TRAVELS - 2);
    s.play(STAR_CHART, json!({}));
    assert_eq!(at(&s), STARSCAPE_EDGE_TRAVELS);
    assert_eq!(traveled(s.last_events()).len(), 2, "no travel past the edge");
    assert_eq!(hand_of(&s, EDGE_OF_THE_UNIVERSE).len(), 1);
}

#[test]
fn b32_star_chart_plays_at_no_mana_and_its_travels_cost_none() {
    for (radiant, nodes) in [(false, STAR_CHART_NODES), (true, RADIANT_STAR_CHART_NODES)] {
        let mut s = chart_board("b32-free", radiant, 0);
        set_map(&mut s, 0);
        s.play(STAR_CHART, json!({}));
        assert_eq!(at(&s), nodes, "radiant {radiant}");
        assert_eq!(s.state().players.p1.mana.current, 0, "no travel charges mana");
    }
}

// ---------------------------------------------------------------------------------------------
// B33 CN Tech, B34 NA Tech
// ---------------------------------------------------------------------------------------------

/// A win-rate table of `(id, wins, games)` rows.
fn win_rates(rows: &[(&str, i32, i32)]) -> WinRateTable {
    WinRateTable {
        cards: rows
            .iter()
            .map(|(id, wins, games)| WinRateRow {
                id: (*id).to_string(),
                wins: *wins,
                games: *games,
            })
            .collect(),
        ..WinRateTable::default()
    }
}

/// `card` played from p1's hand under the win-rate table `rows`.
fn tech_played(seed: &str, card: &str, radiant: bool, rows: &[(&str, i32, i32)]) -> Scenario {
    let mut s = board(seed, json!({ "hand": [on(card, radiant), VANILLA] }), json!({}));
    register_win_rates(win_rates(rows));
    s.play(card, json!({}));
    s
}

/// The cards CN Tech's play added to p1's hand, as (definition, face).
fn cn_tech_adds(seed: &str, radiant: bool, rows: &[(&str, i32, i32)]) -> Vec<(String, bool)> {
    let s = tech_played(seed, CN_TECH, radiant, rows);
    s.last_events()
        .iter()
        .filter_map(|event| match event {
            GameEvent::AddedToHand {
                player: PlayerId::P1,
                instance_id,
                def_id,
                ..
            } => Some((def_id.clone(), s.card(instance_id.as_str()).radiant)),
            _ => None,
        })
        .collect()
}

#[test]
fn b33_cn_tech_with_an_empty_table_adds_lunar_eclipse_on_its_face() {
    for radiant in [false, true] {
        for seed in ["b33-empty-a", "b33-empty-b", "b33-empty-c"] {
            assert_eq!(
                cn_tech_adds(seed, radiant, &[]),
                vec![(LUNAR_ECLIPSE.to_string(), radiant)],
                "{seed}, radiant {radiant}: Lunar Eclipse, never a random card"
            );
        }
    }
}

#[test]
fn b33_cn_tech_with_no_row_of_enough_games_adds_lunar_eclipse() {
    let short = [(VANILLA, CN_TECH_MIN_GAMES - 1, CN_TECH_MIN_GAMES - 1)];
    assert_eq!(
        cn_tech_adds("b33-short", false, &short),
        vec![(LUNAR_ECLIPSE.to_string(), false)]
    );
}

#[test]
fn b33_cn_tech_naming_only_itself_adds_lunar_eclipse() {
    let itself = [(CN_TECH, CN_TECH_MIN_GAMES, CN_TECH_MIN_GAMES)];
    assert_eq!(
        cn_tech_adds("b33-itself", true, &itself),
        vec![(LUNAR_ECLIPSE.to_string(), true)]
    );
}

#[test]
fn b33_cn_tech_with_a_leader_adds_the_leader() {
    let led = [(VANILLA, CN_TECH_MIN_GAMES - 1, CN_TECH_MIN_GAMES)];
    for radiant in [false, true] {
        assert_eq!(
            cn_tech_adds("b33-leader", radiant, &led),
            vec![(VANILLA.to_string(), radiant)],
            "radiant {radiant}"
        );
    }
}

/// The definitions NA Tech's fused card was fused from, sorted.
fn na_tech_ends(seed: &str, radiant: bool, rows: &[(&str, i32, i32)]) -> Vec<String> {
    let s = tech_played(seed, NA_TECH, radiant, rows);
    let fused: Vec<Vec<String>> = s
        .hand(P1)
        .iter()
        .filter_map(|card| {
            find_def(Some(s.state()), &card.def_id)
                .and_then(|def| def.ingredients.as_ref())
                .map(|list| list.iter().map(|ingredient| ingredient.def_id.clone()).collect())
        })
        .collect();
    assert_eq!(fused.len(), 1, "{seed}: one fused card in p1's hand: {fused:?}");
    let mut ends = fused[0].clone();
    ends.sort();
    assert_eq!(ends.len(), 2, "{seed}: two ends: {ends:?}");
    ends
}

#[test]
fn b34_na_tech_with_an_empty_table_fuses_infinite_reserves_and_lunar_eclipse() {
    for radiant in [false, true] {
        for seed in ["b34-empty-a", "b34-empty-b", "b34-empty-c"] {
            assert_eq!(
                na_tech_ends(seed, radiant, &[]),
                vec![LUNAR_ECLIPSE.to_string(), INFINITE_RESERVES.to_string()],
                "{seed}, radiant {radiant}: both fallbacks, never a random card"
            );
        }
    }
}

#[test]
fn b34_na_tech_falls_back_to_infinite_reserves_for_the_end_the_table_cannot_name() {
    // One row qualifies: it is the leader, and nothing is left for the trailer.
    let one = [(VANILLA, CN_TECH_MIN_GAMES - 1, CN_TECH_MIN_GAMES)];
    for seed in ["b34-one-a", "b34-one-b"] {
        assert_eq!(
            na_tech_ends(seed, false, &one),
            vec![VANILLA.to_string(), INFINITE_RESERVES.to_string()]
        );
    }
}

#[test]
fn b34_na_tech_whose_fallback_is_the_named_end_takes_a_random_distinct_card() {
    // The leader is Infinite Reserves itself, so the trailer's fallback would repeat it.
    let reserves = [(INFINITE_RESERVES, CN_TECH_MIN_GAMES - 1, CN_TECH_MIN_GAMES)];
    for seed in ["b34-same-a", "b34-same-b", "b34-same-c"] {
        let ends = na_tech_ends(seed, false, &reserves);
        assert_eq!(
            ends.iter().filter(|end| *end == INFINITE_RESERVES).count(),
            1,
            "{seed}: the two ends stay distinct: {ends:?}"
        );
        let other = ends
            .iter()
            .find(|end| *end != INFINITE_RESERVES)
            .expect("another end");
        assert!(!card_def(other).token, "{seed}: {other} is no token");
    }
}

#[test]
fn b34_na_tech_with_both_ends_named_fuses_them() {
    let both = [
        (VANILLA, CN_TECH_MIN_GAMES - 1, CN_TECH_MIN_GAMES),
        (BIG_D_FENDER, 1, CN_TECH_MIN_GAMES),
    ];
    assert_eq!(
        na_tech_ends("b34-both", false, &both),
        vec![BIG_D_FENDER.to_string(), VANILLA.to_string()]
    );
}

// ---------------------------------------------------------------------------------------------
// B35 Fruit Launcher
// ---------------------------------------------------------------------------------------------

/// p1's Fruit Launcher (lane 1) activated at p1's Mr. Vanilla (lane 2), by the activation
/// `legal_actions` lists for that target; the definitions cast in that step, in order.
fn fruit_cast(seed: &str, radiant: bool) -> Vec<String> {
    let mut s = board(
        seed,
        json!({ "field": [on(FRUIT_LAUNCHER, radiant), VANILLA] }),
        json!({}),
    );
    let launcher = s.unit_at(P1, 1);
    let target = s.unit_at(P1, 2);
    let at_target = Selection::Instance {
        instance_id: target.id.clone(),
    };
    let action = legal_actions(s.state(), P1)
        .into_iter()
        .find(|action| {
            matches!(action, ActionBody::Activate { instance_id, .. } if *instance_id == launcher.id)
                && targets_of(action).contains(&at_target)
        })
        .expect("Fruit Launcher's Activate at the Mr. Vanilla is listed");
    s.act(action, P1, "Fruit Launcher casts a random Fruit");
    s.last_events()
        .iter()
        .filter_map(|event| match event {
            GameEvent::CardPlayed { def_id, .. } => Some(def_id.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn b35_fruit_launcher_can_cast_a_grape_token() {
    for radiant in [false, true] {
        let seed = seed_where(&format!("b35-grape-{radiant}"), |seed| {
            fruit_cast(seed, radiant)
                .first()
                .is_some_and(|fruit| GRAPES.contains(&fruit.as_str()))
        });
        let cast = fruit_cast(&seed, radiant);
        assert!(
            card_def(&cast[0]).token,
            "a Grape is a token, and the pool takes it"
        );
    }
}

#[test]
fn b35_fruit_launcher_casts_nothing_but_a_fruit_of_a_set_that_ships() {
    for radiant in [false, true] {
        for at in 0..12 {
            let seed = format!("b35-only-{radiant}-{at}");
            let cast = fruit_cast(&seed, radiant);
            let fruit = cast.first().unwrap_or_else(|| panic!("{seed}: a Fruit is cast"));
            let def = card_def(fruit);
            assert!(def.tags.contains(&Tag::Fruit), "{seed}: {fruit} is a Fruit");
            assert!(
                SHIPPED_SETS.contains(&def.set),
                "{seed}: {fruit} is of a set that ships"
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// B36 the Resource and Faction tags
// ---------------------------------------------------------------------------------------------

const RESOURCES: [&str; 6] = [
    "stinkyscape-t-reknite",
    "stinkyscape-t-korrelite",
    "stinkyscape-t-gellium",
    "stinkyscape-t-axnit",
    "stinkyscape-t-narcor",
    "stinkyscape-t-red-narcor",
];
const FACTIONS: [&str; 5] = [
    "stinkyscape-159",
    "stinkyscape-161",
    "stinkyscape-095",
    "stinkyscape-160",
    "stinkyscape-162",
];

fn carrying(tag: Tag) -> Vec<String> {
    let mut ids: Vec<String> = CATALOG
        .iter()
        .filter(|(_, def)| def.tags.contains(&tag))
        .map(|(id, _)| id.clone())
        .collect();
    ids.sort();
    ids
}

fn sorted(ids: &[&str]) -> Vec<String> {
    let mut ids: Vec<String> = ids.iter().map(|id| (*id).to_string()).collect();
    ids.sort();
    ids
}

#[test]
fn b36_resource_and_faction_are_tags_spelled_as_the_catalog_writes_them() {
    assert_eq!(Tag::Resource.as_str(), "Resource");
    assert_eq!(Tag::Faction.as_str(), "Faction");
    assert_eq!("Resource".parse::<Tag>().ok(), Some(Tag::Resource));
    assert_eq!("Faction".parse::<Tag>().ok(), Some(Tag::Faction));
    assert_eq!(serde_json::to_value(Tag::Resource).ok(), Some(json!("Resource")));
    assert_eq!(serde_json::to_value(Tag::Faction).ok(), Some(json!("Faction")));
}

#[test]
fn b36_the_two_tags_sit_after_swarm_and_before_token_in_that_order() {
    let at = |tag: Tag| {
        Tag::ALL
            .iter()
            .position(|each| *each == tag)
            .unwrap_or_else(|| panic!("{tag} is in Tag::ALL"))
    };
    assert!(at(Tag::Swarm) < at(Tag::Resource), "Resource after Swarm");
    assert_eq!(
        at(Tag::Faction),
        at(Tag::Resource) + 1,
        "Faction right after Resource"
    );
    assert!(at(Tag::Faction) < at(Tag::Token), "both before Token");
}

#[test]
fn b36_the_census_counts_six_resources_and_five_factions() {
    assert_eq!(carrying(Tag::Resource).len(), RESOURCE_CARDS);
    assert_eq!(carrying(Tag::Faction).len(), FACTION_CARDS);
}

#[test]
fn b36_no_other_card_carries_either_tag() {
    assert_eq!(
        carrying(Tag::Resource),
        sorted(&RESOURCES),
        "only the six Resources"
    );
    assert_eq!(
        carrying(Tag::Faction),
        sorted(&FACTIONS),
        "only the five Faction cards"
    );
    for id in RESOURCES {
        assert!(
            !card_def(id).tags.contains(&Tag::Faction),
            "{id} is no Faction card"
        );
    }
    for id in FACTIONS {
        assert!(!card_def(id).tags.contains(&Tag::Resource), "{id} is no Resource");
    }
}
