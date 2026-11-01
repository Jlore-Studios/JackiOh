//! Issue #2657 (Stinkyscape balance changes II): the engine's new rules. R2657000 Can't Defend, R2657003 a
//! Trade that never draws its own card back while the deck holds another, R2657005 the Buff's and the
//! Nerf's new keyword changes, and R2657006 the numbers KY's Constant reads on top of a Nerf's and a
//! Buff's. R2657001, R2657002 and R2657004 are proven where their cards and verbs are tested
//! (`may_attack_again_sick.rs`, `effects_statuses.rs` and the card files).
//!
//! Fixtures are this file's own: `fx-` defs indexed from 26570 (BUILD §0).

use jackioh_engine::effects::{TuneDirection, TuneRow, applicable_changes, set_number, tune_once};
use jackioh_engine::testkit::PlayerId::P1;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::{spell_def, unit_def};
use crate::rules::fixtures::harness::{both, run_effect, setup_catalog};

/// A 2/2 with Can't Defend.
fn defender() -> CardDef {
    unit_def(26570, json!({ "keywords": [{ "kind": "Can't Defend" }] }))
}

/// A plain 2/2.
fn plain() -> CardDef {
    unit_def(26571, json!({}))
}

/// A 2/2 with Can't attack.
fn grounded() -> CardDef {
    unit_def(26572, json!({ "keywords": [{ "kind": "Can't attack" }] }))
}

/// A (1) Spell that deals no damage.
fn quiet() -> CardDef {
    spell_def(26573, json!({}))
}

/// A (1) Spell with Tradable.
fn tradable() -> CardDef {
    let face = json!({ "keywords": [{ "kind": "Tradable" }], "text": "Tradable" });
    spell_def(26574, json!({ "base": face, "radiant": face }))
}

/// A 2/2 whose script sets the legacy flag `never_defense`, as the scripts kept for v0.5.9e do.
fn flagged() -> CardDef {
    unit_def(26576, json!({}))
}

/// A (1) Spell with Temporary 2.
fn temporary() -> CardDef {
    let face = json!({ "keywords": [{ "kind": "Temporary", "n": 2 }], "text": "Temporary 2" });
    spell_def(26575, json!({ "base": face, "radiant": face }))
}

fn register() {
    setup_catalog();
    let mut catalog = registered_catalog().clone();
    let defs = [
        defender(),
        plain(),
        grounded(),
        quiet(),
        tradable(),
        temporary(),
        flagged(),
    ];
    for def in &defs {
        catalog.insert(def.id.clone(), def.clone());
    }
    register_catalog(catalog);
    let mut scripts = registered_scripts();
    for def in &defs {
        scripts.insert(def.id.clone(), both(Script::default()));
    }
    scripts.insert(
        flagged().id,
        both(Script {
            static_flags: Some(StaticFlags {
                never_defense: Some(true),
                ..StaticFlags::default()
            }),
            ..Script::default()
        }),
    );
    register_scripts(scripts);
}

/// p1's main phase with `p1` over a hand of two plain Units and a deck of three; p2 holds one.
fn board(seed: &str, p1: Value) -> Scenario {
    register();
    let filler = plain().id;
    let mut side = json!({ "hand": [filler, filler], "library": [filler, filler, filler] });
    if let (Value::Object(into), Value::Object(more)) = (&mut side, p1) {
        into.extend(more);
    }
    let mut s = scenario(json!({
        "seed": seed,
        "p1": side,
        "p2": { "hand": [filler], "library": [filler, filler] },
    }));
    s.state_mut().players.p1.auto_end_turn = Some(false);
    s.state_mut().players.p2.auto_end_turn = Some(false);
    s
}

/// One Buff or Nerf on `id`, drawn from the match rng at the state's cursor.
fn tune(s: &mut Scenario, id: &str, direction: TuneDirection) {
    let card = s.card(id).clone();
    let state = s.state_mut();
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(&mut sink, None, HookOptions::default());
        tune_once(&mut ctx, &card, direction, true);
    }
    state.rng_cursor = rng.cursor();
}

/// The keyword kinds the card has now, its tuning included.
fn kinds_of(s: &Scenario, id: &str) -> Vec<KeywordKind> {
    card_keywords(s.state(), s.card(id))
        .iter()
        .map(Keyword::kind)
        .collect()
}

mod r2657000_cant_defend {
    use super::*;

    #[test]
    fn r2657000_a_unit_with_cant_defend_is_never_offered_nor_allowed_a_switch_to_defense() {
        let mut s = board("r2657000-switch", json!({ "field": [defender().id, plain().id] }));
        let held = s.unit_at(P1, 1).id;
        let free = s.unit_at(P1, 2).id;
        let switches: Vec<String> = legal_actions(s.state(), P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::SwitchPosition { instance_id, .. } => Some(instance_id),
                _ => None,
            })
            .collect();
        assert!(!switches.contains(&held), "{switches:?}");
        assert!(switches.contains(&free), "{switches:?}");
        s.expect_refused_with(|s| s.switch_position(held.as_str()), "Defense Position");
        s.switch_position(free.as_str());
        assert_eq!(s.card(free.as_str()).position, Some(Position::Def));
    }

    #[test]
    fn r2657000_one_standing_in_defense_switches_to_attack_at_the_state_check() {
        let mut s = board("r2657000-stand", json!({ "field": [defender().id] }));
        let held = s.unit_at(P1, 1).id;
        // Put there as an effect that ignored the keyword would, then the next state check runs.
        s.card_mut(held.as_str()).position = Some(Position::Def);
        s.end_turn();
        assert_eq!(s.card(held.as_str()).position, Some(Position::Atk));
        assert!(s.events().iter().any(|event| matches!(
            event,
            GameEvent::PositionSwitched { instance_id, position: Position::Atk } if *instance_id == held
        )));
    }

    #[test]
    fn r2657000_the_legacy_flag_still_holds_a_unit_out_of_defense() {
        // R767000: the scripts kept for v0.5.9e set `never_defense`, the flag the keyword took over from.
        let mut s = board("r2657000-flag", json!({ "field": [flagged().id] }));
        let held = s.unit_at(P1, 1).id;
        s.expect_refused_with(|s| s.switch_position(held.as_str()), "Defense Position");
    }
}

mod r2657003_a_trade_never_draws_its_own_card_back {
    use super::*;

    #[test]
    fn r2657003_while_the_deck_holds_another_card_the_draw_is_another_card() {
        for n in 0..40 {
            let mut s = board(&format!("r2657003-{n}"), json!({ "hand": [tradable().id] }));
            let traded = s.card(tradable().id.as_str()).id.clone();
            s.trade(&traded);
            let card = s.card(traded.as_str());
            assert!(
                matches!(card.zone, Zone::Library { .. }),
                "seed {n}: {:?}",
                card.zone
            );
        }
    }

    #[test]
    fn r2657003_into_an_empty_deck_it_is_the_only_card_and_comes_straight_back() {
        let mut s = board(
            "r2657003-empty",
            json!({ "hand": [tradable().id], "library": [] }),
        );
        let traded = s.card(tradable().id.as_str()).id.clone();
        s.trade(&traded);
        assert!(matches!(s.card(traded.as_str()).zone, Zone::Hand { .. }));
    }
}

mod r2657005_the_buffs_and_the_nerfs_keywords {
    use super::*;

    #[test]
    fn r2657005_a_buff_may_give_a_card_of_any_type_flashback() {
        let s = board("r2657005-rows", json!({ "hand": [quiet().id] }));
        let spell = s.card(quiet().id.as_str()).clone();
        assert!(applicable_changes(s.state(), &spell, TuneDirection::Upgrade).contains(&TuneRow::Keyword));
        let mut gained = false;
        for n in 0..40 {
            let mut s = board(
                &format!("r2657005-flashback-{n}"),
                json!({ "hand": [quiet().id] }),
            );
            let id = s.card(quiet().id.as_str()).id.clone();
            tune(&mut s, &id, TuneDirection::Upgrade);
            let kinds = kinds_of(&s, &id);
            assert!(
                kinds.iter().all(|kind| *kind == KeywordKind::Flashback),
                "{kinds:?}"
            );
            gained |= kinds.contains(&KeywordKind::Flashback);
        }
        assert!(gained, "some Buff gave Flashback");
    }

    #[test]
    fn r2657005_a_buff_may_take_cant_attack_or_cant_defend_off_and_a_nerf_never_takes_either() {
        for (def, kind) in [
            (grounded(), KeywordKind::CantAttack),
            (defender(), KeywordKind::CantDefend),
        ] {
            // The removal is one item among the row's additions (the combat pool and Flashback), so
            // it lands about once in two dozen Buffs.
            let mut lost = false;
            for n in 0..200 {
                let mut s = board(&format!("r2657005-off-{n}"), json!({ "field": [def.id] }));
                let id = s.unit_at(P1, 1).id;
                tune(&mut s, &id, TuneDirection::Upgrade);
                lost |= !kinds_of(&s, &id).contains(&kind);
                let mut s = board(&format!("r2657005-kept-{n}"), json!({ "field": [def.id] }));
                let id = s.unit_at(P1, 1).id;
                tune(&mut s, &id, TuneDirection::Degrade);
                assert!(
                    kinds_of(&s, &id).contains(&kind),
                    "{kind:?}: a Nerf never takes it off"
                );
            }
            assert!(lost, "{kind:?}: some Buff took it off");
        }
    }

    #[test]
    fn r2657005_a_nerf_may_give_crater_in_a_hand_never_on_the_field_and_never_cant_attack_or_cant_defend() {
        let mut cratered = false;
        for n in 0..60 {
            let mut s = board(&format!("r2657005-crater-{n}"), json!({ "hand": [plain().id] }));
            let id = s.card(plain().id.as_str()).id.clone();
            tune(&mut s, &id, TuneDirection::Degrade);
            let kinds = kinds_of(&s, &id);
            assert!(kinds.iter().all(|kind| *kind == KeywordKind::Crater), "{kinds:?}");
            cratered |= kinds.contains(&KeywordKind::Crater);

            let mut s = board(&format!("r2657005-field-{n}"), json!({ "field": [plain().id] }));
            let id = s.unit_at(P1, 1).id;
            tune(&mut s, &id, TuneDirection::Degrade);
            assert!(
                kinds_of(&s, &id).is_empty(),
                "no keyword a Nerf adds on the field"
            );

            let mut s = board(&format!("r2657005-spell-{n}"), json!({ "hand": [quiet().id] }));
            let id = s.card(quiet().id.as_str()).id.clone();
            tune(&mut s, &id, TuneDirection::Degrade);
            assert!(kinds_of(&s, &id).is_empty(), "never Crater on a Spell");
        }
        assert!(cratered, "some Nerf gave the Unit in hand Crater");
    }
}

mod r2657006_ky_constants_further_numbers {
    use super::*;

    #[test]
    fn r2657006_a_temporary_cards_count_is_a_number_on_it_and_a_set_writes_it() {
        let mut s = board("r2657006-temporary", json!({ "hand": [temporary().id] }));
        let id = s.card(temporary().id.as_str()).id.clone();
        let numbers = numbers_on(s.state(), s.card(id.as_str()));
        let count = numbers.iter().find(|entry| entry.ref_ == NumberRef::Temporary);
        assert_eq!(count.map(|entry| entry.value), Some(2), "{numbers:?}");
        let effect = set_number(json_as(
            json!({ "instanceId": id, "which": { "kind": "temporary" }, "value": 3 }),
        ));
        run_effect(s.state_mut(), effect, P1);
        assert_eq!(s.card(id.as_str()).temporary, Some(3));
        assert!(card_keywords(s.state(), s.card(id.as_str())).contains(&Keyword::Temporary { n: Some(3) }));
    }

    #[test]
    fn r2657006_the_plague_counters_on_a_card_are_a_number_on_it_and_a_set_writes_them() {
        let mut s = board(
            "r2657006-plague",
            json!({ "field": [{ "def": plain().id, "counters": { "plague": 2 } }] }),
        );
        let id = s.unit_at(P1, 1).id;
        let numbers = numbers_on(s.state(), s.card(id.as_str()));
        let counters = numbers.iter().find(|entry| entry.ref_ == NumberRef::Plague);
        assert_eq!(counters.map(|entry| entry.value), Some(2), "{numbers:?}");
        let effect = set_number(json_as(
            json!({ "instanceId": id, "which": "plague", "value": 3 }),
        ));
        let events = run_effect(s.state_mut(), effect, P1);
        assert_eq!(s.card(id.as_str()).counters.plague, Some(3));
        assert!(events.iter().any(|event| matches!(
            event,
            GameEvent::CounterChanged {
                counter: CounterKind::Plague,
                value: 3,
                ..
            }
        )));
        // A card with neither lists neither.
        let bare = board("r2657006-bare", json!({ "field": [plain().id] }));
        let id = bare.unit_at(P1, 1).id;
        let refs: Vec<NumberRef> = numbers_on(bare.state(), bare.card(id.as_str()))
            .into_iter()
            .map(|entry| entry.ref_)
            .collect();
        assert!(!refs.contains(&NumberRef::Temporary) && !refs.contains(&NumberRef::Plague));
    }
}
