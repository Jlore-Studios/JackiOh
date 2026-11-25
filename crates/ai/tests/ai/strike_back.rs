//! R4402000, #4402: the AI's Taunt walk (`damage_past_taunts_with`, so `face_threat` and `ready_gap`) reads
//! each Taunt's strike back (§4.3). An attacker the strike back destroys still lands that hit (R59) but makes
//! none of its later swings (R636), at a Taunt or the hero, and the wounds a strike back leaves carry to its
//! next swing; a Ranged attacker takes none from a non-Ranged Taunt (R783000). A Frenzy attacker whose hit
//! destroys the Taunt it attacked, and that survives, has its swings back to `attacks_per_turn` (R3543003);
//! a Cleave splash's kill frees none. Each board is played out through `reduce` in the order the walk reads
//! it, and the hero damage the engine deals is the read. `GREEDY_EVAL` reads every board as before.
//!
//! The cards: Mr. Vanilla (Core #8) 4/4, Archivist (Core #30) 4/5, Tommy Tempo (Classic+ #26) 3/3 Taunt,
//! Tmack Trav (Stinkyscape #94) 7/7 Rush Frenzy, Silly Silas (Core #52) 7/7, Zoomerbin Oomen (Core #67) 1/2
//! and Thorium Wall (S+ #65) 0/10 Taunt; Overdrive Projector (S+ #62), awake in p2's backrow, gives every
//! Unit on both sides Rush and Windfury.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, register_cards, unit};

/// Mr. Vanilla, 4/4.
const VANILLA: &str = "core-008";
/// Archivist, 4/5.
const ARCHIVIST: &str = "core-030";
/// Tommy Tempo, 3/3 Taunt.
const TOMMY: &str = "classicplus-026";
/// Tmack Trav, 7/7 Rush Frenzy.
const TMACK: &str = "stinkyscape-094";
/// Silly Silas, 7/7.
const SILAS: &str = "core-052";
/// Zoomerbin Oomen, 1/2.
const OOMEN: &str = "core-067";
/// Thorium Wall, 0/10 Can't attack, Taunt.
const WALL: &str = "stinkyscapeplus-065";
/// Overdrive Projector: awake, every Unit has Rush and Windfury.
const PROJECTOR: &str = "stinkyscapeplus-062";

/// The walk as it read before #4402.
const OFF: EvalWeights = EvalWeights {
    strike_back_survival_read: false,
    ..AI_EVAL
};

/// p2 (the human) on turn, its units ready, against p1's (the AI's) Taunts. Neither turn ends by itself
/// (R82), so a play-out stays on the human's turn.
fn board(seed: &str, p1: Value, p2: Value) -> GameState {
    register_cards();
    let mut state = scenario(json!({ "seed": seed, "active": "p2", "p1": p1, "p2": p2 }))
        .state()
        .clone();
    for player in [AI, HUMAN] {
        state.players[player].auto_end_turn = Some(false);
    }
    state
}

/// `board` with the Projector awake in p2's backrow: p2's Units have Windfury (its controller's alone
/// since issue #4665).
fn projected(seed: &str, p1: Value, p2_field: Value) -> GameState {
    let state = board(
        seed,
        p1,
        json!({ "field": p2_field, "backrow": [{ "def": PROJECTOR, "faceUp": true }] }),
    );
    for unit in active_units_of(&state, HUMAN) {
        assert_eq!(attacks_per_turn(&state, unit), 2, "the Projector gives Windfury");
    }
    state
}

fn grant(state: &mut GameState, player: PlayerId, def_id: &str, keyword: Keyword) {
    let id = active_units_of(state, player)
        .into_iter()
        .find(|unit| unit.def_id == def_id)
        .map(|unit| unit.id.clone())
        .expect("the unit stands on the field");
    find_instance_mut(state, &id)
        .expect("the unit")
        .granted_keywords
        .push(keyword);
}

/// The human's `def` attacks each of `targets` in turn (`None` is the AI's hero).
fn swings(state: &GameState, def: &str, targets: &[Option<&str>]) -> GameState {
    let attacker = unit(state, HUMAN, def).id.clone();
    let mut after = state.clone();
    for target in targets {
        after = act(
            &after,
            HUMAN,
            &ActionBody::Attack {
                attacker_id: attacker.clone(),
                target_id: target.map_or_else(|| format!("hero-{AI}"), str::to_owned),
            },
        );
    }
    after
}

/// What the play-out dealt the AI's hero.
fn dealt(before: &GameState, after: &GameState) -> i32 {
    assert_eq!(
        (after.active, after.turn),
        (HUMAN, before.turn),
        "the play-out stays on the human's turn"
    );
    before.players[AI].hero.health - after.players[AI].hero.health
}

/// Whether the human's `def` may still declare an attack.
fn may_attack(state: &GameState, def: &str) -> bool {
    let Some(attacker) = active_units_of(state, HUMAN)
        .into_iter()
        .find(|unit| unit.def_id == def)
        .map(|unit| unit.id.clone())
    else {
        return false;
    };
    legal_actions(state, HUMAN)
        .iter()
        .any(|action| matches!(action, ActionBody::Attack { attacker_id, .. } if *attacker_id == attacker))
}

/// On the human's own turn, `ready_gap` is the life the play-out leaves the AI.
fn gap_matches(state: &GameState, after: &GameState) {
    assert_eq!(
        ready_gap(state, HUMAN),
        f64::from(after.players[AI].hero.health),
        "ready_gap is the gap the engine allows"
    );
}

/// Board 1: the Projector; p2's Vanilla against p1's Vanilla with Taunt.
fn windfury_trade() -> GameState {
    let mut state = projected(
        "ai-strike-back-windfury-trade",
        json!({ "field": [VANILLA] }),
        json!([VANILLA]),
    );
    grant(&mut state, AI, VANILLA, Keyword::Taunt);
    state
}

/// Board 2: the Projector; p2's Archivist against p1's Vanilla with Taunt.
fn windfury_survivor() -> GameState {
    let mut state = projected(
        "ai-strike-back-windfury-survivor",
        json!({ "field": [VANILLA] }),
        json!([ARCHIVIST]),
    );
    grant(&mut state, AI, VANILLA, Keyword::Taunt);
    state
}

/// Board 3: p2's Vanilla with Ranged and Windfury against Tommy.
fn ranged_windfury() -> GameState {
    let mut state = board(
        "ai-strike-back-ranged-windfury",
        json!({ "field": [TOMMY] }),
        json!({ "field": [VANILLA] }),
    );
    grant(&mut state, HUMAN, VANILLA, Keyword::Ranged);
    grant(&mut state, HUMAN, VANILLA, Keyword::Windfury);
    state
}

/// Board 4: Tmack Trav against Tommy.
fn tmack_survives() -> GameState {
    board(
        "ai-strike-back-tmack-survives",
        json!({ "field": [TOMMY] }),
        json!({ "field": [TMACK] }),
    )
}

/// Board 5: Tmack Trav against Silly Silas at 7/3 with Taunt.
fn tmack_struck_down() -> GameState {
    let mut state = board(
        "ai-strike-back-tmack-struck-down",
        json!({ "field": [{ "def": SILAS, "damage": 4 }] }),
        json!({ "field": [TMACK] }),
    );
    grant(&mut state, AI, SILAS, Keyword::Taunt);
    state
}

/// Board 6: Tmack Trav against Tommys in lanes 1 and 2.
fn tmack_two_tommys() -> GameState {
    board(
        "ai-strike-back-two-tommys",
        json!({ "field": [{ "def": TOMMY, "lane": 1 }, { "def": TOMMY, "lane": 2 }] }),
        json!({ "field": [TMACK] }),
    )
}

/// Board 7: Tmack Trav against Tommy (lane 1) and Vanilla with Taunt (lane 2).
fn tmack_wounds_carry() -> GameState {
    let mut state = board(
        "ai-strike-back-wounds-carry",
        json!({ "field": [{ "def": TOMMY, "lane": 1 }, { "def": VANILLA, "lane": 2 }] }),
        json!({ "field": [TMACK] }),
    );
    grant(&mut state, AI, VANILLA, Keyword::Taunt);
    state
}

/// Board 8: p2's Vanilla with Frenzy and Cleave against p1's Oomen with Taunt and Divine Shield (lane 1)
/// and Tommy (lane 2).
fn cleave_splash_kill() -> GameState {
    let mut state = board(
        "ai-strike-back-cleave-splash",
        json!({ "field": [{ "def": OOMEN, "lane": 1 }, { "def": TOMMY, "lane": 2 }] }),
        json!({ "field": [VANILLA] }),
    );
    grant(&mut state, AI, OOMEN, Keyword::Taunt);
    grant(&mut state, AI, OOMEN, Keyword::DivineShield);
    grant(&mut state, HUMAN, VANILLA, Keyword::Frenzy);
    grant(&mut state, HUMAN, VANILLA, Keyword::Cleave);
    state
}

/// Board 8's control: p2's Vanilla with Frenzy alone against Tommy.
fn frenzy_vanilla() -> GameState {
    let mut state = board(
        "ai-strike-back-frenzy-vanilla",
        json!({ "field": [TOMMY] }),
        json!({ "field": [VANILLA] }),
    );
    grant(&mut state, HUMAN, VANILLA, Keyword::Frenzy);
    state
}

/// Board 10: the Projector; p2's Vanilla with Divine Shield against p1's Vanilla with Taunt.
fn windfury_shield() -> GameState {
    let mut state = projected(
        "ai-strike-back-windfury-shield",
        json!({ "field": [VANILLA] }),
        json!([VANILLA]),
    );
    grant(&mut state, AI, VANILLA, Keyword::Taunt);
    grant(&mut state, HUMAN, VANILLA, Keyword::DivineShield);
    state
}

/// Board 11: Tmack Trav with Divine Shield against two Silly Silases at 7/3, each with Taunt.
fn tmack_shield_spent() -> GameState {
    let mut state = board(
        "ai-strike-back-shield-spent",
        json!({ "field": [{ "def": SILAS, "damage": 4 }, { "def": SILAS, "damage": 4 }] }),
        json!({ "field": [TMACK] }),
    );
    let silases: Vec<String> = active_units_of(&state, AI)
        .iter()
        .map(|unit| unit.id.clone())
        .collect();
    for id in &silases {
        find_instance_mut(&mut state, id)
            .expect("Silas")
            .granted_keywords
            .push(Keyword::Taunt);
    }
    grant(&mut state, HUMAN, TMACK, Keyword::DivineShield);
    state
}

/// Board 9: the Projector; Tmack Trav against Thorium Wall.
fn windfury_frenzy() -> GameState {
    projected(
        "ai-strike-back-windfury-frenzy",
        json!({ "field": [WALL] }),
        json!([TMACK]),
    )
}

#[test]
fn r4402000_a_windfury_unit_its_first_strike_back_destroys_swings_no_more() {
    let state = windfury_trade();
    assert_eq!(face_threat(&state, HUMAN), 0);
    assert_eq!(
        face_threat_with(&state, HUMAN, &OFF),
        4,
        "before #4402: kill the Taunt, then 4 face"
    );
    let taunt = unit(&state, AI, VANILLA).id.clone();
    let after = swings(&state, VANILLA, &[Some(&taunt)]);
    assert!(active_units_of(&after, AI).is_empty(), "the Taunt is destroyed");
    assert!(
        active_units_of(&after, HUMAN).is_empty(),
        "and so is its attacker"
    );
    assert_eq!(dealt(&state, &after), face_threat(&state, HUMAN));
    gap_matches(&state, &after);
}

#[test]
fn r4402000_a_windfury_unit_that_survives_the_trade_swings_at_the_hero() {
    let state = windfury_survivor();
    assert_eq!(face_threat(&state, HUMAN), 4);
    let taunt = unit(&state, AI, VANILLA).id.clone();
    let after = swings(&state, ARCHIVIST, &[Some(&taunt), None]);
    assert!(active_units_of(&after, AI).is_empty());
    assert_eq!(dealt(&state, &after), 4);
    assert_eq!(dealt(&state, &after), face_threat(&state, HUMAN));
    gap_matches(&state, &after);
}

#[test]
fn r4402000_a_ranged_windfury_unit_takes_no_strike_back_and_keeps_both_swings() {
    let state = ranged_windfury();
    assert_eq!(face_threat(&state, HUMAN), 4);
    let tommy = unit(&state, AI, TOMMY).id.clone();
    let after = swings(&state, VANILLA, &[Some(&tommy), None]);
    assert!(active_units_of(&after, AI).is_empty());
    assert_eq!(unit(&after, HUMAN, VANILLA).damage, 0, "R783000: no strike back");
    assert_eq!(dealt(&state, &after), face_threat(&state, HUMAN));
}

#[test]
fn r4402000_tmack_trav_swings_again_after_a_kill_it_survives() {
    let state = tmack_survives();
    assert_eq!(face_threat(&state, HUMAN), 7);
    assert_eq!(
        face_threat_with(&state, HUMAN, &OFF),
        0,
        "before #4402: the Taunt held it off"
    );
    let tommy = unit(&state, AI, TOMMY).id.clone();
    let after = swings(&state, TMACK, &[Some(&tommy), None]);
    assert_eq!(dealt(&state, &after), 7);
    assert_eq!(dealt(&state, &after), face_threat(&state, HUMAN));
    gap_matches(&state, &after);
}

#[test]
fn r4402000_tmack_trav_swings_no_more_when_the_strike_back_destroys_it() {
    let state = tmack_struck_down();
    assert_eq!(face_threat(&state, HUMAN), 0);
    let silas = unit(&state, AI, SILAS).id.clone();
    let after = swings(&state, TMACK, &[Some(&silas)]);
    assert!(active_units_of(&after, AI).is_empty(), "Silas is destroyed");
    assert!(
        active_units_of(&after, HUMAN).is_empty(),
        "and its strike back destroys Tmack Trav"
    );
    assert_eq!(dealt(&state, &after), face_threat(&state, HUMAN));
}

#[test]
fn r4402000_frenzy_breaks_two_taunts_then_swings_at_the_hero() {
    let state = tmack_two_tommys();
    assert_eq!(face_threat(&state, HUMAN), 7);
    assert_eq!(face_threat_with(&state, HUMAN, &OFF), 0);
    let tommys: Vec<String> = active_units_of(&state, AI)
        .iter()
        .map(|unit| unit.id.clone())
        .collect();
    let [first, second] = tommys.as_slice() else {
        panic!("two Tommys");
    };
    let after = swings(&state, TMACK, &[Some(first), Some(second), None]);
    assert!(active_units_of(&after, AI).is_empty());
    assert_eq!(dealt(&state, &after), 7);
    assert_eq!(dealt(&state, &after), face_threat(&state, HUMAN));
    gap_matches(&state, &after);
}

#[test]
fn r4402000_a_frenzy_attackers_wounds_carry_to_its_next_taunt() {
    let state = tmack_wounds_carry();
    // Tommy's 3 leaves Tmack Trav at 4 health, and the Vanilla's 4 then destroys it.
    assert_eq!(face_threat(&state, HUMAN), 0);
    let (tommy, vanilla) = (
        unit(&state, AI, TOMMY).id.clone(),
        unit(&state, AI, VANILLA).id.clone(),
    );
    let after = swings(&state, TMACK, &[Some(&tommy), Some(&vanilla)]);
    assert!(active_units_of(&after, AI).is_empty());
    assert!(
        active_units_of(&after, HUMAN).is_empty(),
        "Tmack Trav dies to the second strike back"
    );
    assert_eq!(dealt(&state, &after), face_threat(&state, HUMAN));
}

#[test]
fn r4402000_a_cleave_splash_kill_frees_no_frenzy_swing() {
    let state = cleave_splash_kill();
    // The Oomen's shield takes the hit and the splash destroys Tommy: the Taunt attacked stands.
    assert_eq!(face_threat(&state, HUMAN), 0);
    let oomen = unit(&state, AI, OOMEN).id.clone();
    let after = swings(&state, VANILLA, &[Some(&oomen)]);
    assert!(
        !active_units_of(&after, AI)
            .iter()
            .any(|unit| unit.def_id == TOMMY),
        "the splash destroys Tommy"
    );
    assert!(
        active_units_of(&after, AI)
            .iter()
            .any(|unit| unit.def_id == OOMEN),
        "the Oomen stands"
    );
    assert!(
        !may_attack(&after, VANILLA),
        "no Frenzy swing for a splash's kill"
    );
    assert_eq!(dealt(&state, &after), face_threat(&state, HUMAN));

    // The control: the same Vanilla with Frenzy alone, destroying the Tommy it attacked, swings again.
    let control = frenzy_vanilla();
    assert_eq!(face_threat(&control, HUMAN), 4);
    let tommy = unit(&control, AI, TOMMY).id.clone();
    let after = swings(&control, VANILLA, &[Some(&tommy), None]);
    assert_eq!(dealt(&control, &after), 4);
    assert_eq!(dealt(&control, &after), face_threat(&control, HUMAN));
}

#[test]
fn r4402000_frenzy_gives_a_windfury_unit_its_whole_exertion_back() {
    let state = windfury_frenzy();
    // Two swings break the Wall, and its kill gives Tmack Trav both of its swings back.
    assert_eq!(face_threat(&state, HUMAN), 14);
    assert_eq!(face_threat_with(&state, HUMAN, &OFF), 0);
    let wall = unit(&state, AI, WALL).id.clone();
    let after = swings(&state, TMACK, &[Some(&wall), Some(&wall), None, None]);
    assert!(active_units_of(&after, AI).is_empty());
    assert_eq!(dealt(&state, &after), 14);
    assert_eq!(dealt(&state, &after), face_threat(&state, HUMAN));
    gap_matches(&state, &after);
}

#[test]
fn r4402000_a_divine_shield_takes_one_strike_back_and_stays_popped() {
    // The shield takes the Taunt's 4, so the Windfury Vanilla survives its trade and swings at the hero.
    let state = windfury_shield();
    assert_eq!(face_threat(&state, HUMAN), 4);
    let taunt = unit(&state, AI, VANILLA).id.clone();
    let after = swings(&state, VANILLA, &[Some(&taunt), None]);
    assert!(active_units_of(&after, AI).is_empty());
    assert_eq!(dealt(&state, &after), 4);
    assert_eq!(dealt(&state, &after), face_threat(&state, HUMAN));
    gap_matches(&state, &after);

    // The first Silas's 7 pops Tmack Trav's shield and its kill frees a swing, but the second Silas's 7
    // then destroys it: the shield it spent does not come back for the next strike back.
    let state = tmack_shield_spent();
    assert_eq!(face_threat(&state, HUMAN), 0);
    let silases: Vec<String> = active_units_of(&state, AI)
        .iter()
        .map(|unit| unit.id.clone())
        .collect();
    let [first, second] = silases.as_slice() else {
        panic!("two Silases");
    };
    let after = swings(&state, TMACK, &[Some(first), Some(second)]);
    assert!(active_units_of(&after, AI).is_empty());
    assert!(
        active_units_of(&after, HUMAN).is_empty(),
        "the second strike back destroys Tmack Trav"
    );
    assert_eq!(dealt(&state, &after), face_threat(&state, HUMAN));
}

#[test]
fn r4402000_greedy_eval_reads_every_board_as_before() {
    const { assert!(AI_EVAL.strike_back_survival_read) };
    const { assert!(!GREEDY_EVAL.strike_back_survival_read) };
    let boards: [(GameState, i32); 12] = [
        (windfury_trade(), 4),
        (windfury_survivor(), 4),
        (ranged_windfury(), 4),
        (tmack_survives(), 0),
        (tmack_struck_down(), 0),
        (tmack_two_tommys(), 0),
        (tmack_wounds_carry(), 0),
        (cleave_splash_kill(), 0),
        (frenzy_vanilla(), 0),
        (windfury_frenzy(), 0),
        (windfury_shield(), 4),
        (tmack_shield_spent(), 0),
    ];
    for (state, old) in &boards {
        assert_eq!(face_threat_with(state, HUMAN, &GREEDY_EVAL), *old);
        assert_eq!(face_threat_with(state, HUMAN, &OFF), *old);
    }
}
