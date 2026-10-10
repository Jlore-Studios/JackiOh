//! Fixtures for the Meditative batch 18 engine tests (R1120–R1127): one card per behaviour the
//! batch's systems add — a combat-only attack modifier, a Poisonous one, an exile-on-damage unit and
//! the two judge faces — each on both faces alike, so `{ radiant: true }` never silently changes what
//! the fixture means.
//!
//! Each def echoes the §8 card named in its doc comment, trimmed to the one property its name
//! promises. The real cards, with their own tests, live in `crates/cards`.

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use jackioh_engine::testkit::*;

use super::catalog::vanilla_deck;
use super::harness::setup_catalog;

/// TS `def(overrides)`: a Core Common card with empty faces, `overrides` over it.
fn def(overrides: Value) -> CardDef {
    let mut literal = json!({
        "set": "Core",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": "" },
        "radiant": { "keywords": [], "text": "" },
    });
    if let (Some(target), Value::Object(extra)) = (literal.as_object_mut(), overrides) {
        for (key, value) in extra {
            target.insert(key, value);
        }
    }
    json_as(literal)
}

/// The same script on both faces.
fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

/// Meditative #52's modifier as a fixture: +2 while any attacker strikes any Unit (R1120).
pub static herald: LazyLock<CardDef> = LazyLock::new(|| {
    def(json!({
        "id": "cj-herald",
        "index": "2001",
        "name": "Herald (+2 combat-only, fixture)",
        "type": "Field Spell",
        "base": { "keywords": [], "text": "+2 combat-only" },
        "radiant": { "keywords": [], "text": "+2 combat-only" },
    }))
});

/// Meditative #52's Radiant face as a fixture: its controller's attackers get +2 and Poisonous
/// (R1120).
pub static herald_poison: LazyLock<CardDef> = LazyLock::new(|| {
    def(json!({
        "id": "cj-herald-poison",
        "index": "2002",
        "name": "Herald (+2 Poisonous, own attackers, fixture)",
        "type": "Field Spell",
        "base": { "keywords": [], "text": "+2 Poisonous, own attackers" },
        "radiant": { "keywords": [], "text": "+2 Poisonous, own attackers" },
    }))
});

/// Meditative #72 as a fixture: a 2/2 that exiles on damage (R1124).
pub static banisher: LazyLock<CardDef> = LazyLock::new(|| {
    def(json!({
        "id": "cj-banisher",
        "index": "2003",
        "name": "Banisher (exile on damage, fixture)",
        "type": "Unit",
        "cost": 2,
        "base": { "attack": 2, "health": 2, "keywords": [], "text": "exile on damage" },
        "radiant": { "attack": 2, "health": 2, "keywords": [], "text": "exile on damage" },
    }))
});

/// Meditative #72's Radiant face as a fixture: a 4/2 with Cleave that exiles on damage (R1124).
pub static banisher_cleave: LazyLock<CardDef> = LazyLock::new(|| {
    def(json!({
        "id": "cj-banisher-cleave",
        "index": "2008",
        "name": "Banisher (Cleave, exile on damage, fixture)",
        "type": "Unit",
        "cost": 3,
        "base": { "attack": 4, "health": 2, "keywords": [{ "kind": "Cleave" }], "text": "Cleave, exile on damage" },
        "radiant": { "attack": 4, "health": 2, "keywords": [{ "kind": "Cleave" }], "text": "Cleave, exile on damage" },
    }))
});

/// Meditative #71's base face as a fixture: judges plays (R1125).
pub static judge: LazyLock<CardDef> = LazyLock::new(|| {
    def(json!({
        "id": "cj-judge",
        "index": "2004",
        "name": "Judge (judges plays, fixture)",
        "type": "Field Spell",
        "cost": 2,
        "base": { "keywords": [], "text": "judges plays" },
        "radiant": { "keywords": [], "text": "judges plays" },
    }))
});

/// Meditative #71's Radiant face as a fixture: judges plays and hears emotes (R1125, R1127).
pub static judge_emote: LazyLock<CardDef> = LazyLock::new(|| {
    def(json!({
        "id": "cj-judge-emote",
        "index": "2005",
        "name": "Judge (judges plays, hears emotes, fixture)",
        "type": "Field Spell",
        "cost": 2,
        "base": { "keywords": [], "text": "judges plays, hears emotes" },
        "radiant": { "keywords": [], "text": "judges plays, hears emotes" },
    }))
});

/// A Trap with no text, so a face-down play keeps its judgement under R227 (R1125).
pub static snare: LazyLock<CardDef> = LazyLock::new(|| {
    def(json!({
        "id": "cj-snare",
        "index": "2009",
        "name": "Snare (blank Trap, fixture)",
        "type": "Trap",
        "base": { "keywords": [], "text": "nothing" },
        "radiant": { "keywords": [], "text": "nothing" },
    }))
});

/// A 1/1 (Radiant 2/2) both players can afford to play, so the judge has candidates (R1125).
pub static cheap: LazyLock<CardDef> = LazyLock::new(|| {
    def(json!({
        "id": "cj-cheap",
        "index": "2006",
        "name": "Cheap Body (fixture)",
        "type": "Unit",
        "base": { "attack": 1, "health": 1, "keywords": [], "text": "1/1" },
        "radiant": { "attack": 2, "health": 2, "keywords": [], "text": "2/2" },
    }))
});

/// A 5/5 for 6 (Radiant 10/10): better stats, a worse rate, so the judge prefers the cheap body
/// (R1125).
pub static dear: LazyLock<CardDef> = LazyLock::new(|| {
    def(json!({
        "id": "cj-dear",
        "index": "2007",
        "name": "Dear Body (fixture)",
        "type": "Unit",
        "cost": 6,
        "base": { "attack": 5, "health": 5, "keywords": [], "text": "5/5 for 6" },
        "radiant": { "attack": 10, "health": 10, "keywords": [], "text": "10/10 for 6" },
    }))
});

/// The scripts of `COMBAT_JUDGE_DEFS`, by def id.
pub static COMBAT_JUDGE_SCRIPTS: LazyLock<Vec<(String, CardScripts)>> = LazyLock::new(|| {
    vec![
        (
            herald.id.clone(),
            both(Script {
                attack_mods: Some(attack_mod_hook(|_args| AttackMod {
                    attack: 2,
                    poisonous: false,
                })),
                ..Script::default()
            }),
        ),
        (
            herald_poison.id.clone(),
            both(Script {
                attack_mods: Some(attack_mod_hook(|args| {
                    if args.attacker.controller != args.self_.controller {
                        return AttackMod::default();
                    }
                    AttackMod {
                        attack: 2,
                        poisonous: true,
                    }
                })),
                ..Script::default()
            }),
        ),
        (
            banisher.id.clone(),
            both(Script {
                static_flags: Some(StaticFlags {
                    exiles_on_damage: Some(true),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            }),
        ),
        (
            banisher_cleave.id.clone(),
            both(Script {
                static_flags: Some(StaticFlags {
                    exiles_on_damage: Some(true),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            }),
        ),
        (
            judge.id.clone(),
            both(Script {
                static_flags: Some(StaticFlags {
                    judges_plays: Some(true),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            }),
        ),
        (
            judge_emote.id.clone(),
            both(Script {
                static_flags: Some(StaticFlags {
                    judges_plays: Some(true),
                    hears_emotes: Some(true),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            }),
        ),
        (cheap.id.clone(), both(Script::default())),
        (dear.id.clone(), both(Script::default())),
        (snare.id.clone(), both(Script::default())),
    ]
});

/// The fixture catalog with these cards, registered on top of the shared one.
pub fn register_combat_judge_fixtures() -> CardDefs {
    setup_catalog();
    let mut defs: CardDefs = registered_catalog().clone();
    for card in [
        &*herald,
        &*herald_poison,
        &*banisher,
        &*banisher_cleave,
        &*judge,
        &*judge_emote,
        &*cheap,
        &*dear,
        &*snare,
    ] {
        defs.insert(card.id.clone(), card.clone());
    }
    register_catalog(defs.clone());
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(COMBAT_JUDGE_SCRIPTS.clone());
    register_scripts(merged);
    defs
}

/// A game in setup with these fixtures registered; decks default to the vanilla fixture decks.
pub fn combat_judge_game(seed: &str, decks: Option<(Vec<String>, Vec<String>)>) -> GameState {
    register_combat_judge_fixtures();
    create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks: decks.unwrap_or_else(|| (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21))),
        ..CreateGameOptions::default()
    })
}
