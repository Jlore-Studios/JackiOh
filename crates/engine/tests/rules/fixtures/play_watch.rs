//! Test-only cards for the rest-of-game play watcher (issue #1863, R1863000): "for the rest of the
//! game, the first time you play each …" (`watch_first_plays`). Each does it through the engine's own
//! verbs, as a card script would, so the engine never imports `crates/cards`; the Stinkyscape Great
//! Replacement Theory (S #44) is the card that uses it.
//!
//! Ids are prefixed `pw-` and indexed from 18630, so they collide with no other fixture file's.

use std::sync::LazyLock;

use jackioh_engine::effects::{WatchFirstPlaysArgs, add_to_hand, cast_new, watch_first_plays};
use jackioh_engine::testkit::*;

use super::harness::both;

/// The watch's badge.
pub const LABEL: &str = "pw watch";

fn card(index: u32, name: &str) -> CardDef {
    let face = json!({ "keywords": [], "text": name });
    json_as(json!({
        "id": format!("pw-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (play watch fixture)"),
        "set": "Core",
        "type": "Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face,
        "radiant": face,
    }))
}

/// A Spell: "For the rest of the game, the first time you play each of alpha and beta, add a copy of
/// it to your hand." Radiant: the copies are Radiant.
pub static WATCH: LazyLock<CardDef> = LazyLock::new(|| card(18630, "watch"));
/// A Spell with no script, watched.
pub static ALPHA: LazyLock<CardDef> = LazyLock::new(|| card(18631, "alpha"));
/// A Spell with no script, watched.
pub static BETA: LazyLock<CardDef> = LazyLock::new(|| card(18632, "beta"));
/// A Spell with no script, never watched.
pub static GAMMA: LazyLock<CardDef> = LazyLock::new(|| card(18633, "gamma"));
/// A Spell: "Cast a beta" (R70).
pub static CASTER: LazyLock<CardDef> = LazyLock::new(|| card(18634, "caster"));

/// The step the watch re-enters for each watched card's first resolved play.
const ANSWERED: &str = "answered";

/// The watch: its Cry sets the watcher, its step adds the card the watcher answered.
fn watch_script(radiant: bool) -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![watch_first_plays(WatchFirstPlaysArgs {
                def_ids: vec![ALPHA.id.clone(), BETA.id.clone()],
                step: ANSWERED.to_string(),
                label: LABEL.to_string(),
                hook: None,
                data: None,
            })]
        })),
        resume: IndexMap::from([(
            ANSWERED,
            hook(move |ctx| {
                let Some(def_id) = ctx.data.get("defId").and_then(Value::as_str) else {
                    return vec![];
                };
                vec![add_to_hand(json_as(
                    json!({ "defId": def_id, "radiant": radiant }),
                ))]
            }),
        )]),
        ..Script::default()
    }
}

static SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (
            WATCH.id.clone(),
            CardScripts {
                base: watch_script(false),
                radiant: watch_script(true),
            },
        ),
        (
            CASTER.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| vec![cast_new(json_as(json!({ "def": BETA.id })))])),
                ..Script::default()
            }),
        ),
    ])
});

pub static PW_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        WATCH.clone(),
        ALPHA.clone(),
        BETA.clone(),
        GAMMA.clone(),
        CASTER.clone(),
    ]
});

/// The engine's fixture catalog (`harness::setup_catalog`) with these cards and scripts on top.
pub fn register_play_watch_fixtures() {
    super::harness::setup_catalog();
    let mut defs: CardDefs = registered_catalog().clone();
    for def in PW_DEFS.iter() {
        defs.insert(def.id.clone(), def.clone());
    }
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(SCRIPTS.clone());
    register_scripts(merged);
}
