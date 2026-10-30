//! R1824000: every walk of a side's acting cards takes a Unit a carrier holds once, a Unit for every
//! rule (R446), its text read wherever a field card's is; a Unit standing in a backrow zone (R1081000)
//! is read once too, its draw limit and trigger multiplier asked once a read.
//!
//! Fixtures are this file's own: a `cu-` carrier and `fx-` Units indexed from 18241 (BUILD §0).

use std::cell::Cell;

use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::unit_def;
use crate::rules::fixtures::field::{flush, playing};
use crate::rules::fixtures::harness::{act, both, in_hand, put, sink_for, slot};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

const CARRIER_LANE: i32 = 2;
/// The lane a Unit stands in the backrow by itself (R1081000).
const STANDING_LANE: i32 = 4;
const CREDIT: i32 = 3;
const ECHO: i32 = 1;
const REVEAL_ECHO: i32 = 1;
const PLAGUE: i32 = 2;
const MANA: i32 = 10;

// Fixtures.

/// A Field Spell that carries a Unit (R446).
fn carrier() -> CardDef {
    json_as(json!({
        "id": "cu-carrier",
        "index": "18241",
        "name": "cu-carrier",
        "set": "Core",
        "type": "Field Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": "carrier" },
        "radiant": { "keywords": [], "text": "carrier" },
    }))
}
fn rider() -> CardDef {
    unit_def(18242, json!({}))
}
/// Sets the price of its controller's Units to (0) (E15).
fn pricer() -> CardDef {
    unit_def(18243, json!({}))
}
/// Lets its controller play any card from their graveyard (E11).
fn grantor() -> CardDef {
    unit_def(18244, json!({}))
}
/// #38 Quickstriker's lasting effect.
fn striker() -> CardDef {
    unit_def(18245, json!({}))
}
/// MD-B2's "no player generates mana naturally" (R941).
fn drought() -> CardDef {
    unit_def(18246, json!({}))
}
/// Jlarna's credit line, Tributed at the end of a turn it went unused (R1223, R1225).
fn lender() -> CardDef {
    unit_def(18247, json!({}))
}
/// #79 Twinspell's "the next Spell you play gains Echo +1" (R209).
fn twin() -> CardDef {
    unit_def(18248, json!({}))
}
/// Radiant Paranoia's "Spells you reveal have Echo +1" (R1045).
fn paranoid() -> CardDef {
    unit_def(18249, json!({}))
}
/// A Flying Unit standing in a backrow zone (R1081000), its draw limit and multiplier counting each ask.
fn flyer() -> CardDef {
    unit_def(18250, json!({ "keywords": [{ "kind": "Flying" }] }))
}

thread_local! {
    static ASKED: Cell<u32> = const { Cell::new(0) };
}

fn count_ask() {
    ASKED.with(|asked| asked.set(asked.get() + 1));
}

fn asked() -> u32 {
    ASKED.with(Cell::get)
}

fn reset_asks() {
    ASKED.with(|asked| asked.set(0));
}

fn flagged(flags: Value) -> CardScripts {
    both(Script {
        static_flags: Some(json_as(flags)),
        ..Script::default()
    })
}

fn scripts() -> Vec<(String, CardScripts)> {
    vec![
        (carrier().id, flagged(json!({ "carrier": true }))),
        (rider().id, both(Script::default())),
        (
            pricer().id,
            both(Script {
                cost_aura: Some(read_hook(|_args| {
                    json_as::<Vec<CostAura>>(json!([{ "whose": "yours", "types": ["Unit"], "setTo": 0 }]))
                })),
                ..Script::default()
            }),
        ),
        (
            grantor().id,
            both(Script {
                graveyard_play: Some(read_hook(|_args| vec![GraveyardPlayPermission::default()])),
                ..Script::default()
            }),
        ),
        (striker().id, flagged(json!({ "quickstriker": true }))),
        (drought().id, flagged(json!({ "noNaturalMana": true }))),
        (
            lender().id,
            flagged(json!({ "creditLine": CREDIT, "creditLapses": true })),
        ),
        (twin().id, flagged(json!({ "echoGrant": ECHO }))),
        (
            paranoid().id,
            both(Script {
                reveal_echo: Some(read_hook(|_args| REVEAL_ECHO)),
                ..Script::default()
            }),
        ),
        (
            flyer().id,
            both(Script {
                draw_limit: Some(read_hook(|_args| {
                    count_ask();
                    Vec::new()
                })),
                turn_hook_extra: Some(read_hook(|_args| {
                    count_ask();
                    config::TRIGGER_EXTRA_NONE
                })),
                ..Script::default()
            }),
        ),
    ]
}

// Harness.

fn board(seed: &str) -> GameState {
    let mut state = playing(&format!("carried-reads-{seed}"));
    let mut catalog = registered_catalog().clone();
    for def in [
        carrier(),
        rider(),
        pricer(),
        grantor(),
        striker(),
        drought(),
        lender(),
        twin(),
        paranoid(),
        flyer(),
    ] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    for (id, script) in scripts() {
        registry.insert(id, script);
    }
    register_scripts(registry);
    flush(&mut state, P1, MANA);
    state
}

/// The carrier in p1's backrow and a `def_id` Unit standing on it: the state, the carrier and the Unit.
fn carrying(seed: &str, def_id: &str) -> (GameState, CardInstance, CardInstance) {
    let mut state = board(seed);
    let carrier = put(
        &mut state,
        &carrier().id,
        slot(P1, Row::Backrow, CARRIER_LANE),
        json!({}),
    );
    let unit = put(
        &mut state,
        def_id,
        slot(P1, Row::Backrow, CARRIER_LANE),
        json!({}),
    );
    assert!(zones::is_carried(&state, &unit), "the Unit is not carried");
    (state, carrier, unit)
}

fn by(player: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(player),
        ..Default::default()
    }
}

fn ids<'a>(cards: impl IntoIterator<Item = &'a CardInstance>) -> Vec<String> {
    cards.into_iter().map(|card| card.id.clone()).collect()
}

fn apply(state: &mut GameState, effect: Effect) {
    let mut sink = sink_for(state);
    (effect.apply)(&mut make_context(&mut sink, None, by(P1)));
}

mod r1824000_every_walk_of_a_sides_acting_cards_takes_a_carried_unit_once {
    use super::*;

    #[test]
    fn acting_on_lists_each_acting_card_once_in_either_order() {
        let (mut state, carrier, carried) = carrying("acting-on", &rider().id);
        let unit = put(&mut state, &rider().id, slot(P1, Row::Units, 1), json!({}));
        let standing = put(
            &mut state,
            &flyer().id,
            slot(P1, Row::Backrow, STANDING_LANE),
            json!({}),
        );
        assert!(
            zones::stands_in_backrow(&state, &standing),
            "the flyer does not stand"
        );
        assert_eq!(
            ids(zones::acting_on(&state, P1, zones::StandingAt::AmongUnits)),
            vec![
                unit.id.clone(),
                carried.id.clone(),
                standing.id.clone(),
                carrier.id.clone()
            ]
        );
        assert_eq!(
            ids(zones::acting_on(&state, P1, zones::StandingAt::ItsLane)),
            vec![unit.id, carried.id, carrier.id, standing.id]
        );
    }

    #[test]
    fn a_standing_units_draw_limit_and_multiplier_are_asked_once() {
        let mut state = board("standing-asked-once");
        put(
            &mut state,
            &flyer().id,
            slot(P1, Row::Backrow, STANDING_LANE),
            json!({}),
        );
        reset_asks();
        draw::draw_limit_of(&state, P1);
        assert_eq!(asked(), 1);
        reset_asks();
        multipliers::extra_runs(&state, P1, multipliers::Multiplied::TurnHooks);
        assert_eq!(asked(), 1);
    }

    #[test]
    fn your_units_on_the_field_in_a_card_scope_reach_the_carried_unit() {
        let (mut state, carrier, rider) = carrying("scope", &rider().id);
        let mut sink = sink_for(&mut state);
        let ctx = make_context(&mut sink, None, by(P1));
        let units = effects::cards_in_card_scope(
            &ctx,
            &json_as(json!({ "side": "self", "zones": ["field"], "rows": ["units"] })),
            None,
        );
        assert_eq!(ids(units.iter().map(|scoped| &scoped.card)), vec![rider.id]);
        let backrow = effects::cards_in_card_scope(
            &ctx,
            &json_as(json!({ "side": "self", "zones": ["field"], "rows": ["backrow"] })),
            None,
        );
        assert_eq!(ids(backrow.iter().map(|scoped| &scoped.card)), vec![carrier.id]);
    }

    #[test]
    fn plague_counters_on_the_carried_unit_count_on_the_field() {
        let (mut state, carrier, rider) = carrying("plague", &rider().id);
        assert_eq!(place_plague_on(&mut sink_for(&mut state), &rider, PLAGUE), PLAGUE);
        assert_eq!(plague_on_field(&state, None::<PlayerId>), PLAGUE);
        assert_eq!(ids(permanents_on_field(&state, P1)), vec![rider.id, carrier.id]);
    }

    #[test]
    fn the_carried_units_cost_aura_prices_a_play() {
        let (mut state, _, _) = carrying("price", &pricer().id);
        let held = in_hand(&mut state, &rider().id, P1, 1).remove(0);
        assert_eq!(cost_rules::cost_auras_for(&state, P1, &held).len(), 1);
        assert_eq!(mana::play_cost(&state, &held), 0);
    }

    #[test]
    fn the_carried_unit_grants_its_graveyard_play() {
        let (state, _, grantor) = carrying("graveyard", &grantor().id);
        let grants = graveyard_play::graveyard_grants_of(&state, P1);
        assert_eq!(ids(grants.iter().map(|grant| &grant.source)), vec![grantor.id]);
    }

    #[test]
    fn the_carried_units_quickstriker_hits_on_the_next_play() {
        let (mut state, _, _) = carrying("quickstriker", &striker().id);
        let health = state.players[P2].hero.health;
        let hand = in_hand(&mut state, &rider().id, P1, 2);
        let play = |state: &GameState, card: &CardInstance, lane: i32| {
            act(
                state,
                json!({
                    "type": "play", "instanceId": card.id, "zone": { "row": "units", "lane": lane },
                    "playerId": "p1"
                }),
            )
        };
        let state = play(&state, &hand[0], 1);
        assert_eq!(state.players[P2].hero.health, health);
        assert!(query::granted_combo_live(&state, P1));
        let state = play(&state, &hand[1], 3);
        assert_eq!(
            state.players[P2].hero.health,
            health - config::QUICKSTRIKER_COMBO_MULTIPLE.base
        );
    }

    #[test]
    fn the_carried_unit_stops_natural_mana() {
        let (state, _, _) = carrying("drought", &drought().id);
        assert!(!mana::natural_mana_on(&state));
    }

    #[test]
    fn the_carried_units_credit_line_opens_and_lapses() {
        let (state, _, lender) = carrying("credit", &lender().id);
        assert_eq!(
            credit::credit_terms(&state, P1),
            Some(credit::CreditTerms {
                limit: CREDIT,
                instalments: 1
            })
        );
        assert_eq!(credit::credit_lapsing(&state, P1), vec![lender.id]);
    }

    #[test]
    fn the_carried_units_echo_grant_installs_its_rider() {
        let (mut state, _, twin) = carrying("echo-grant", &twin().id);
        modifiers::install_lasting_modifiers(&mut sink_for(&mut state));
        assert!(state.players[P1].mods.iter().any(|modifier| matches!(
            &modifier.kind,
            ModifierKind::EchoNextSpell { amount, source_id: Some(id) } if *amount == ECHO && *id == twin.id
        )));
    }

    #[test]
    fn the_carried_units_reveal_echo_counts() {
        let (state, _, _) = carrying("reveal-echo", &paranoid().id);
        assert_eq!(alt_play::reveal_echo_bonus(&state, P1), REVEAL_ECHO);
    }

    #[test]
    fn a_random_make_radiant_on_your_field_may_pick_the_carried_unit() {
        let mut state = board("make-radiant");
        put(
            &mut state,
            &carrier().id,
            slot(P1, Row::Backrow, CARRIER_LANE),
            json!({ "radiant": true }),
        );
        let rider = put(
            &mut state,
            &rider().id,
            slot(P1, Row::Backrow, CARRIER_LANE),
            json!({}),
        );
        apply(
            &mut state,
            effects::set_radiant_random(json_as(json!({ "zones": "field" }))),
        );
        assert!(
            find_instance(&state, &rider.id)
                .expect("the rider is gone")
                .radiant
        );
    }

    #[test]
    fn fuse_onto_your_card_offers_the_carried_unit() {
        let (mut state, _, carried) = carrying("fuse-onto", &rider().id);
        let ingredient = put(&mut state, &rider().id, slot(P1, Row::Units, 1), json!({}));
        apply(
            &mut state,
            effects::fuse_onto_your_card(json_as(json!({
                "target": { "of": "instance", "instanceId": ingredient.id },
                "from": ["field"],
                "otherwise": "nothing",
            }))),
        );
        let pending = state.pending.as_ref().expect("no prompt opened");
        let keys: Vec<String> = pending.options.iter().map(|option| option.key.clone()).collect();
        assert_eq!(keys, vec![format!("instance:{}", carried.id)]);
    }
}
