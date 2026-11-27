//! The mana ceiling (SPEC §2.3, R5148000, issue #5148): a player holds at most `MANA_CEILING` (15) mana,
//! current or maximum, temporary mana included, and an X-cost card chooses or counts an X of at most 15
//! (an X that is no price, Lil' Kenny's count, is not capped). Costs are not capped, so a card that
//! costs more is paid only by infinite mana (R3548004) or a credit line (R1223), as Core #100 Ceaseless
//! Void at (16) or more under S+ #56 Impact Reactor.

use jackioh_engine::config::MANA_CEILING;
use jackioh_engine::play_choices::legal_x_values;
use jackioh_engine::testkit::*;
use jackioh_engine::tuning::x_of;

use PlayerId::{P1, P2};

const FILLER: &str = "core-005";
const MENACE: &str = "core-019"; // Midrange Menace: the Unit given away
const DIVIDEND: &str = "core-024"; // Efficiency Dividend: X; deal X damage, heal 3X or X mana next turn
const VOID: &str = "core-100"; // Ceaseless Void: a Unit that costs more than the ceiling
const ME_NO_LIKEY: &str = "meditative-006"; // Me no Likey: give a Unit away; gain its cost (Radiant twice it)
const LIL_KENNY: &str = "meditativeplus-020-1"; // Lil' Kenny: a token whose X is its summoner's count
const JLARNA: &str = "meditative-089"; // Jlarna: Credit line 4
const REACTOR: &str = "stinkyscapeplus-056"; // Impact Reactor: infinite mana once awake

fn setup(opts: Value) -> Scenario {
    jackioh_cards::register_all();
    let mut s = scenario(opts);
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

fn mana_of(s: &Scenario, player: PlayerId) -> (i32, i32) {
    let mana = &s.state().players[player].mana;
    (mana.current, mana.max)
}

#[test]
fn r5148000_the_ceiling_is_15() {
    assert_eq!(MANA_CEILING, 15);
}

#[test]
fn r5148000_temporary_mana_stops_at_15_and_what_passes_it_is_lost() {
    // Radiant Me no Likey gains twice the Menace's (3): 12 + 6 is 18, held as 15.
    let mut s = setup(json!({
        "seed": "r5148000-float",
        "p1": { "hand": [{ "def": ME_NO_LIKEY, "radiant": true }, FILLER], "mana": 12, "field": [MENACE] },
        "p2": { "hand": [FILLER] },
    }));
    let menace = s.unit(P1, 1).expect("the Menace").id.clone();
    s.play(ME_NO_LIKEY, json!({ "targets": pick::card(&s, menace.as_str()) }));
    s.expect_mana(P1, MANA_CEILING);
    // A plain gain past it is held as 15 too, and a loss after it comes off the 15.
    let side = &mut s.state_mut().players[P1];
    mana::gain_mana(side, 100);
    assert_eq!(side.mana.current, MANA_CEILING);
    mana::gain_mana(side, -4);
    assert_eq!(side.mana.current, MANA_CEILING - 4);
}

#[test]
fn r5148000_a_refresh_gives_at_most_15_whatever_its_rider() {
    let mut s =
        setup(json!({ "seed": "r5148000-refresh", "p1": { "hand": [FILLER] }, "p2": { "hand": [FILLER] } }));
    let side = &mut s.state_mut().players[P1];
    side.mana.next_turn_mod = 40;
    mana::refresh_mana_with_bonus(side, true, 7);
    assert_eq!(side.mana.current, MANA_CEILING);
    assert_eq!(side.mana.next_turn_mod, 0, "the rider is spent");
    // Without natural mana (MD-B2) the rider alone is given, held under the ceiling as well.
    side.mana.next_turn_mod = 40;
    mana::refresh_mana_with(side, false);
    assert_eq!(side.mana.current, MANA_CEILING);
}

#[test]
fn r5148000_max_mana_never_passes_15() {
    let mut s =
        setup(json!({ "seed": "r5148000-max", "p1": { "hand": [FILLER] }, "p2": { "hand": [FILLER] } }));
    let side = &mut s.state_mut().players[P1];
    side.mana.perm_mod = 30;
    side.stolen_mana_crystals = Some(9);
    assert_eq!(mana::max_mana_for(side), MANA_CEILING);
    mana::refresh_mana(side);
    assert_eq!((side.mana.current, side.mana.max), (MANA_CEILING, MANA_CEILING));
}

#[test]
fn r5148000_a_crystal_steal_holds_the_thief_at_15() {
    let mut s =
        setup(json!({ "seed": "r5148000-steal", "p1": { "hand": [FILLER] }, "p2": { "hand": [FILLER] } }));
    {
        let thief = &mut s.state_mut().players[P1];
        thief.mana.perm_mod = 10;
        thief.mana.max = mana::max_mana_for(thief);
        thief.mana.current = MANA_CEILING;
    }
    mana::move_mana_crystals(s.state_mut(), P1, P2, mana::CrystalGain::PastTheCap);
    assert_eq!(mana_of(&s, P1), (MANA_CEILING, MANA_CEILING));
}

#[test]
fn r5148000_r1223_an_x_stops_at_15_though_a_credit_line_lends_past_it() {
    // 15 held and 4 more lent: the picker still offers 1 to 15, and a play for X = 16 is refused.
    let mut s = setup(json!({
        "seed": "r5148000-x",
        "p1": { "hand": [DIVIDEND, FILLER], "backrow": [JLARNA], "mana": 15 },
        "p2": { "hand": [FILLER] },
    }));
    assert_eq!(
        jackioh_engine::credit::spendable_mana(s.state(), P1),
        MANA_CEILING + 4
    );
    let dividend = s
        .hand(P1)
        .into_iter()
        .find(|card| card.def_id == DIVIDEND)
        .expect("the Dividend");
    assert_eq!(
        legal_x_values(s.state(), P1, &dividend),
        (1..=MANA_CEILING).collect::<Vec<_>>()
    );
    let health = s.state().players[P2].hero.health;
    s.expect_refused_with(
        |s| {
            s.play(
                DIVIDEND,
                json!({ "x": MANA_CEILING + 1, "modes": ["damage"], "targets": pick::hero(P2) }),
            )
        },
        "X is at most 15",
    );
    s.play(
        DIVIDEND,
        json!({ "x": MANA_CEILING, "modes": ["damage"], "targets": pick::hero(P2) }),
    );
    assert_eq!(s.state().players[P2].hero.health, health - MANA_CEILING);
}

#[test]
fn r5148000_an_x_cost_cards_x_counts_at_most_15_a_set_x_included_and_an_x_that_is_no_price_is_not_capped() {
    let s = setup(json!({
        "seed": "r5148000-count",
        "p1": { "hand": [DIVIDEND, LIL_KENNY] },
        "p2": { "hand": [FILLER] },
    }));
    let mut card = s
        .hand(P1)
        .into_iter()
        .find(|card| card.def_id == DIVIDEND)
        .expect("the Dividend");
    card.x = Some(40);
    assert_eq!(mana::counted_x(s.state(), &card), MANA_CEILING);
    card.x = None;
    card.fixed_x = Some(22);
    assert_eq!(mana::counted_x(s.state(), &card), MANA_CEILING);
    // Lil' Kenny costs (1): its X/X is the count of KY's Competition's right answers, not mana.
    let mut kenny = s
        .hand(P1)
        .into_iter()
        .find(|card| card.def_id == LIL_KENNY)
        .expect("Lil' Kenny");
    kenny.x = Some(30);
    assert_eq!(mana::counted_x(s.state(), &kenny), 30);
    assert_eq!(x_of(&kenny), 30);
}

#[test]
fn r5148000_r3548004_infinite_mana_still_plays_a_ceaseless_void_costing_16_or_more() {
    let mut s = setup(json!({
        "seed": "r5148000-void",
        "p1": { "hand": [VOID, FILLER], "backrow": [REACTOR], "mana": MANA_CEILING },
        "p2": { "hand": [FILLER] },
    }));
    assert_eq!(s.view(P1).you.infinite_mana, Some(true), "the Reactor is awake");
    let void = s
        .hand(P1)
        .into_iter()
        .find(|card| card.def_id == VOID)
        .expect("the Void");
    assert!(
        mana::play_cost(s.state(), &void) > MANA_CEILING,
        "the Void costs 16 or more"
    );
    assert!(s.can_play(VOID));
    s.play(VOID, json!({ "zone": 4 }));
    s.expect_in_zone(VOID, "field");
    assert_eq!(mana_of(&s, P1).0, MANA_CEILING, "nothing spent");
}

#[test]
fn r5148000_without_infinite_mana_a_void_costing_16_or_more_is_out_of_reach_of_15() {
    let s = setup(json!({
        "seed": "r5148000-no-void",
        "p1": { "hand": [VOID, FILLER], "mana": MANA_CEILING },
        "p2": { "hand": [FILLER] },
    }));
    assert!(!s.can_play(VOID));
}

#[test]
fn r5148000_r3548004_under_infinite_mana_an_x_is_still_chosen_out_of_at_most_15() {
    let s = setup(json!({
        "seed": "r5148000-infinite-x",
        "p1": { "hand": [DIVIDEND, FILLER], "backrow": [REACTOR], "mana": MANA_CEILING },
        "p2": { "hand": [FILLER] },
    }));
    assert_eq!(s.view(P1).you.infinite_mana, Some(true));
    let dividend = s
        .hand(P1)
        .into_iter()
        .find(|card| card.def_id == DIVIDEND)
        .expect("the Dividend");
    assert_eq!(
        legal_x_values(s.state(), P1, &dividend),
        (1..=MANA_CEILING).collect::<Vec<_>>()
    );
}
