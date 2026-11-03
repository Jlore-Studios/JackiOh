//! Issue #1748's Buff priorities over the real cards (SPEC §6.3 Nerf and Buff): on the field a Nerf or
//! a Buff never draws a card's cost nor a number only its Cry reads (R1748000); a Buff gives a Field
//! Spell Indestructible, and a Spell or a Field Spell that deals damage Pierce or Lifesteal (R1748001);
//! on the field a Buff places a Plague Counter on a Field Spell that uses its own, and a Nerf removes
//! one (R1748002). Since issue #2436 a Buff may give a Field Spell Immutable too (R2436000).

use jackioh_engine::effects::{TuneDirection, TuneRow, applicable_changes, degrade, destroy, upgrade};
use jackioh_engine::testkit::*;

use super::scenario;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

const CRAWLER: &str = "classic-053"; // Plague Crawler: (1) Unit; Cry `tokens`, lasting `draw`.
const LIVING_BOMB: &str = "classic-062"; // (1) Field Spell, no numbers.
const PLANTATION: &str = "classic-074"; // Corpse Plantation: (2) Field Spell, `usesPlague`.
const ECLIPSE: &str = "core-035"; // Lunar Eclipse: (1) Spell, deals 3 damage, `dealsDamage`.
const FIELD_OF_DREAMS: &str = "core-076"; // (2) Spell that deals no damage.
const INTROSPECTION: &str = "meditative-007"; // (4) Field Spell: End of turn ... deal 2 damage to each enemy, `dealsDamage`.
const ARMORED: &str = "core-025"; // A 7/7 Unit with Armor 7.
const FILLER: &str = "core-005";

fn apply_for_p1(s: &mut Scenario, effects: &[Effect]) -> Vec<GameEvent> {
    let mut rng = create_rng(&s.state().seed, s.state().rng_cursor);
    let mut events: Vec<GameEvent> = Vec::new();
    {
        let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
        {
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(P1),
                    ..HookOptions::default()
                },
            );
            apply_effects(effects, &mut ctx);
        }
        settle(&mut sink, SettleOptions::default());
    }
    s.state_mut().rng_cursor = rng.cursor();
    events
}

fn tune(s: &mut Scenario, card: &str, direction: TuneDirection) -> Vec<GameEvent> {
    let args = json_as(json!({ "target": { "of": "instance", "instanceId": s.card(card).id } }));
    let effect = match direction {
        TuneDirection::Upgrade => upgrade(args),
        TuneDirection::Degrade => degrade(args),
    };
    apply_for_p1(s, &[effect])
}

fn rows(s: &Scenario, card: &str, direction: TuneDirection) -> Vec<TuneRow> {
    applicable_changes(s.state(), s.card(card), direction)
}

fn steppable_keys(s: &Scenario, card: &str, direction: TuneDirection) -> Vec<String> {
    steppable_params(s.state(), s.card(card), direction)
        .into_iter()
        .map(|item| item.param.key)
        .collect()
}

fn has(s: &Scenario, card: &str, kind: KeywordKind) -> bool {
    has_keyword(&unit_view(s.state(), s.card(card)).keywords, kind)
}

fn give_pierce(s: &mut Scenario, card: &str) {
    s.card_mut(card).tuning = Some(Tuning {
        add_keywords: Some(vec![Keyword::Pierce]),
        ..Tuning::default()
    });
}

fn count(s: &Scenario, event_type: GameEventType) -> usize {
    s.events()
        .iter()
        .filter(|event| event.event_type() == event_type)
        .count()
}

/// Each `damage` event's amount dealt and the part of the hit its target's Armor took (`absorbed`, R1360).
fn hits(s: &Scenario) -> Vec<(i32, i32)> {
    s.events()
        .iter()
        .filter_map(|event| match event {
            GameEvent::Damage { amount, absorbed, .. } => Some((*amount, *absorbed)),
            _ => None,
        })
        .collect()
}

fn living_bomb(seed: &str) -> Scenario {
    scenario(json!({
        "seed": seed,
        "p1": { "hand": [FILLER], "backrow": [{ "def": LIVING_BOMB, "faceUp": true }] },
        "p2": { "hand": [FILLER] },
    }))
}

#[test]
fn r1748000_a_cry_number_is_out_of_reach_on_the_field() {
    let s = scenario(json!({
        "p1": { "hand": [CRAWLER, FILLER] },
        "p2": { "hand": [FILLER] },
    }));
    assert_eq!(
        steppable_keys(&s, CRAWLER, TuneDirection::Upgrade),
        ["tokens", "draw"]
    );
    assert!(rows(&s, CRAWLER, TuneDirection::Upgrade).contains(&TuneRow::Cost));

    let s = scenario(json!({
        "p1": { "hand": [FILLER], "field": [CRAWLER] },
        "p2": { "hand": [FILLER] },
    }));
    for direction in [TuneDirection::Upgrade, TuneDirection::Degrade] {
        assert!(
            !rows(&s, CRAWLER, direction).contains(&TuneRow::Cost),
            "{direction:?}"
        );
    }
    assert_eq!(steppable_keys(&s, CRAWLER, TuneDirection::Upgrade), ["draw"]);
}

#[test]
fn r1748001_a_buff_gives_a_field_spell_indestructible() {
    // R2436000: its keyword row draws Indestructible or Immutable, on a seed that draws Indestructible.
    let seed = seed_where("living-bomb-indestructible", |seed| {
        let mut s = living_bomb(seed);
        tune(&mut s, LIVING_BOMB, TuneDirection::Upgrade);
        has(&s, LIVING_BOMB, KeywordKind::Indestructible)
    });
    let mut s = living_bomb(&seed);
    assert_eq!(rows(&s, LIVING_BOMB, TuneDirection::Upgrade), [TuneRow::Keyword]);
    assert!(!has(&s, LIVING_BOMB, KeywordKind::Indestructible));

    let events = tune(&mut s, LIVING_BOMB, TuneDirection::Upgrade);

    assert!(has(&s, LIVING_BOMB, KeywordKind::Indestructible));
    assert!(events.iter().any(|event| matches!(
        event,
        GameEvent::Upgraded {
            change: TuningChange::Keyword {
                keyword: Keyword::Indestructible,
                added: true,
                ..
            },
            ..
        }
    )));
    // Indestructible already, it has Immutable left to gain (R2436000).
    assert_eq!(rows(&s, LIVING_BOMB, TuneDirection::Upgrade), [TuneRow::Keyword]);
    let id = s.card(LIVING_BOMB).id.clone();
    apply_for_p1(
        &mut s,
        &[destroy(json_as(
            json!({ "target": { "of": "instance", "instanceId": id } }),
        ))],
    );
    assert_eq!(s.card(LIVING_BOMB).zone.z(), ZoneName::Field);
    // Its next Buff has Immutable alone to give, with no draw (R129).
    tune(&mut s, LIVING_BOMB, TuneDirection::Upgrade);
    assert!(has(&s, LIVING_BOMB, KeywordKind::Immutable));
}

#[test]
fn r2436000_a_buff_may_give_a_field_spell_immutable_and_nothing_tunes_it_after() {
    let seed = seed_where("living-bomb-immutable", |seed| {
        let mut s = living_bomb(seed);
        tune(&mut s, LIVING_BOMB, TuneDirection::Upgrade);
        has(&s, LIVING_BOMB, KeywordKind::Immutable)
    });
    let mut s = living_bomb(&seed);
    assert!(!immutable_in_play(s.state(), s.card(LIVING_BOMB)));

    let events = tune(&mut s, LIVING_BOMB, TuneDirection::Upgrade);

    assert!(has(&s, LIVING_BOMB, KeywordKind::Immutable));
    assert!(!has(&s, LIVING_BOMB, KeywordKind::Indestructible));
    assert!(events.iter().any(|event| matches!(
        event,
        GameEvent::Upgraded {
            change: TuningChange::Keyword {
                keyword: Keyword::Immutable,
                added: true,
                ..
            },
            ..
        }
    )));
    // Face-up on the field it is Immutable in play: no Spell, Field Spell or Field Trap targets it (R781001).
    assert!(immutable_in_play(s.state(), s.card(LIVING_BOMB)));
    // And no Buff or Nerf changes it again (R23): neither has a row, and another Buff leaves it as it is.
    for direction in [TuneDirection::Upgrade, TuneDirection::Degrade] {
        assert!(rows(&s, LIVING_BOMB, direction).is_empty(), "{direction:?}");
    }
    let tuned = s.card(LIVING_BOMB).tuning.clone();
    tune(&mut s, LIVING_BOMB, TuneDirection::Upgrade);
    assert_eq!(s.card(LIVING_BOMB).tuning, tuned);
}

#[test]
fn r1748001_a_damage_spell_may_gain_pierce_or_lifesteal_and_its_lifesteal_heals() {
    let s = scenario(json!({
        "p1": { "hand": [ECLIPSE, FIELD_OF_DREAMS, FILLER] },
        "p2": { "hand": [FILLER] },
    }));
    assert!(rows(&s, ECLIPSE, TuneDirection::Upgrade).contains(&TuneRow::Keyword));
    // A Spell that deals no damage still has a keyword row since #2657: Flashback, which a Buff may give
    // any card (R2657005), and never Pierce or Lifesteal.
    assert!(rows(&s, FIELD_OF_DREAMS, TuneDirection::Upgrade).contains(&TuneRow::Keyword));

    // A Buff draws one of the two; given Lifesteal, its 3 damage heals its caster 3.
    let mut s = scenario(json!({
        "seed": "eclipse-lifesteal",
        "p1": { "hand": [ECLIPSE, FILLER], "health": 20 },
        "p2": { "hand": [FILLER] },
    }));
    s.card_mut(ECLIPSE).tuning = Some(Tuning {
        add_keywords: Some(vec![Keyword::Lifesteal]),
        ..Tuning::default()
    });
    s.play(
        ECLIPSE,
        json!({ "targets": [{ "pick": "hero", "player": "p2" }] }),
    );
    s.expect_health(P2, 27);
    s.expect_health(P1, 23);
}

#[test]
fn r1748001_a_spell_a_buff_gives_pierce_goes_through_a_heros_armor() {
    let eclipse_at_p2 = |pierce: bool| {
        let mut s = scenario(json!({
            "p1": { "hand": [ECLIPSE, FILLER] },
            "p2": { "hand": [FILLER], "armor": 3 },
        }));
        if pierce {
            give_pierce(&mut s, ECLIPSE);
        }
        s.play(
            ECLIPSE,
            json!({ "targets": [{ "pick": "hero", "player": "p2" }] }),
        );
        s
    };

    let mut s = eclipse_at_p2(false);
    s.expect_health(P2, 30);
    assert!(hits(&s).is_empty());
    assert_eq!(count(&s, GameEventType::DamageAbsorbed), 1);

    let mut s = eclipse_at_p2(true);
    s.expect_health(P2, 27);
    assert_eq!(hits(&s), [(3, 0)]);
    assert_eq!(count(&s, GameEventType::DamageAbsorbed), 0);
}

#[test]
fn r1748001_a_field_spell_a_buff_gives_pierce_goes_through_a_units_armor() {
    // Introspection's end of turn deals 2 to each enemy, with no Pierce of its own (The Power to Punish's has
    // it, #2179).
    let introspection_at_armored = |pierce: bool| {
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "library": [FILLER, FILLER], "backrow": [{ "def": INTROSPECTION, "faceUp": true }] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER], "field": [ARMORED] },
        }));
        if pierce {
            give_pierce(&mut s, INTROSPECTION);
        }
        s.end_turn();
        s
    };
    let unit_hits = |s: &Scenario| -> Vec<(i32, i32)> {
        let armored = s.card(ARMORED).id.clone();
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage {
                    target_id,
                    amount,
                    absorbed,
                    ..
                } if *target_id == armored => Some((*amount, *absorbed)),
                _ => None,
            })
            .collect()
    };

    // Unbuffed, the Unit's Armor 7 takes the whole of its 1 damage.
    let s = introspection_at_armored(false);
    assert_eq!(s.stats(ARMORED).health, 7);
    assert!(unit_hits(&s).is_empty());
    assert_eq!(count(&s, GameEventType::DamageAbsorbed), 1);

    // Given Pierce, it lands and the Armor takes nothing.
    let s = introspection_at_armored(true);
    assert_eq!(s.stats(ARMORED).health, 6);
    assert_eq!(unit_hits(&s), [(1, 0)]);
    assert_eq!(count(&s, GameEventType::DamageAbsorbed), 0);
}

#[test]
fn r1748002_a_buff_places_a_plague_counter_and_a_nerf_removes_one() {
    // Granted Indestructible, it has Immutable alone to gain (R2436000): a Buff draws among its keyword
    // row, its Activate's count (the X row) and the plague row, on a seed that draws the plague row.
    let plantation = |seed: &str| {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [FILLER], "backrow": [{ "def": PLANTATION, "faceUp": true }] },
            "p2": { "hand": [FILLER] },
        }));
        s.card_mut(PLANTATION)
            .granted_keywords
            .push(Keyword::Indestructible);
        s
    };
    let seed = seed_where("plantation-plague", |seed| {
        let mut s = plantation(seed);
        tune(&mut s, PLANTATION, TuneDirection::Upgrade);
        s.card(PLANTATION).counters.plague.is_some()
    });
    let mut s = plantation(&seed);
    assert_eq!(
        rows(&s, PLANTATION, TuneDirection::Upgrade),
        [TuneRow::Keyword, TuneRow::X, TuneRow::Plague]
    );

    let events = tune(&mut s, PLANTATION, TuneDirection::Upgrade);

    assert_eq!(s.card(PLANTATION).counters.plague, Some(1));
    let id = s.card(PLANTATION).id.clone();
    let told: Vec<&GameEvent> = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                GameEvent::CounterChanged { .. } | GameEvent::Upgraded { .. }
            )
        })
        .collect();
    assert!(matches!(
        told.as_slice(),
        [
            GameEvent::CounterChanged { instance_id: placed_on, counter: CounterKind::Plague, value: 1, placed: Some(1) },
            GameEvent::Upgraded { instance_id: upgraded, change: TuningChange::Plague { delta: 1 }, hidden_from: None, .. },
        ] if *placed_on == id && *upgraded == id
    ));

    let mut s = scenario(json!({
        "p1": { "hand": [FILLER], "backrow": [{ "def": PLANTATION, "faceUp": true, "counters": { "plague": 2 } }] },
        "p2": { "hand": [FILLER] },
    }));
    assert_eq!(rows(&s, PLANTATION, TuneDirection::Degrade), [TuneRow::Plague]);
    let events = tune(&mut s, PLANTATION, TuneDirection::Degrade);
    assert_eq!(s.card(PLANTATION).counters.plague, Some(1));
    assert!(events.iter().any(|event| matches!(
        event,
        GameEvent::Degraded {
            change: TuningChange::Plague { delta: -1 },
            ..
        }
    )));
    let s = scenario(json!({
        "p1": { "hand": [FILLER], "backrow": [{ "def": PLANTATION, "faceUp": true }] },
        "p2": { "hand": [FILLER] },
    }));
    assert!(rows(&s, PLANTATION, TuneDirection::Degrade).is_empty());

    let s = scenario(json!({
        "p1": { "hand": [PLANTATION, FILLER] },
        "p2": { "hand": [FILLER] },
    }));
    for direction in [TuneDirection::Upgrade, TuneDirection::Degrade] {
        assert!(
            !rows(&s, PLANTATION, direction).contains(&TuneRow::Plague),
            "{direction:?}"
        );
    }
}
