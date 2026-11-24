//! Audit (Stinkyscape+ cards, first half: S+ #1 to #100).

use jackioh_engine::testkit::*;

const P1: PlayerId = PlayerId::P1;

fn scenario_with_cards(opts: serde_json::Value) -> Scenario {
    jackioh_cards::register_all();
    scenario(opts)
}

fn upgraded(events: &[GameEvent]) -> usize {
    events.iter().filter(|event| event.event_type().as_str() == "upgraded").count()
}

/// S+ #24 Forced Propagation, Radiant: "Shuffle 10 copies of it into your deck, then Buff them."
/// Each resolution Buffs the copies it made, once (R3557001: "marks the new cards by the cast and then
/// Buffs each of them once"). Under Echo +1 (S+ #62 Overdrive Projector) the Spell resolves twice, so
/// twenty copies, each Buffed once: twenty Buffs.
#[test]
fn forced_propagation_radiant_echo_buffs_each_copy_once() {
    const PROPAGATION: &str = "stinkyscapeplus-024";
    const PROJECTOR: &str = "stinkyscapeplus-062";
    const BODY: &str = "core-019";
    let mut s = scenario_with_cards(json!({
        "seed": "audit-fp",
        "p1": {
            "hand": [{ "def": PROPAGATION, "radiant": true }, "core-005"],
            "field": [BODY],
            "backrow": [{ "def": PROJECTOR, "faceUp": true }],
            "library": ["core-005", "core-005", "core-005"],
            "mana": 10,
        },
        "p2": { "hand": ["core-005"], "library": ["core-005", "core-005", "core-005", "core-005"] },
    }));
    let body = s.unit_at(P1, 1).id;
    s.play(PROPAGATION, pick::targets(pick::id(&body)));
    while s.state().pending.is_some() {
        s.answer(pick::id(&body));
    }
    let copies = s.state().players[P1].library.iter().filter(|card| card.def_id == BODY).count();
    assert_eq!(copies, 20, "Echo +1: two resolutions of ten copies");
    assert_eq!(upgraded(s.events()), 20, "each copy is Buffed once, by the resolution that made it");
}

/// S+ #74 Galaxia: "Aura: The first Unit you play each turn gets +{attack} Attack." R3568000: "the
/// first Unit her controller plays each turn gets a permanent +3 Attack ... once its play resolves".
/// TG #28 Occidental ("Cry: Destroy all permanents in the rightmost lane"), played into the rightmost
/// lane, is the turn's first Unit play and dies to its own Cry. The Unit played after it is the turn's
/// second and gets nothing.
#[test]
fn galaxia_buffs_only_the_first_unit_played_each_turn() {
    const GALAXIA: &str = "stinkyscapeplus-074";
    const OCCIDENTAL: &str = "traininggrounds-028";
    const BODY: &str = "stand-in (1) 4/4";
    let mut s = scenario_with_cards(json!({
        "p1": {
            "hand": [OCCIDENTAL, BODY],
            "field": [{ "def": GALAXIA, "lane": 1 }],
            "library": ["core-005", "core-005", "core-005"],
            "mana": 10,
        },
        "p2": { "hand": ["core-005"], "library": ["core-005", "core-005", "core-005", "core-005"] },
    }));
    s.play(OCCIDENTAL, json!({ "zone": 5 }));
    s.expect_in_zone(OCCIDENTAL, "graveyard");
    s.play(BODY, json!({ "zone": 2 }));
    let second = s.unit_at(P1, 2).id;
    assert_eq!(s.stats(second.as_str()).attack, 4, "the second Unit played this turn is not the first");
}
