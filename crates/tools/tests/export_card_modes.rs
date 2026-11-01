//! Writes `apps/web/src/game/cardModes.json`: every card's declared Choose one (#2646), the options
//! of each `mode` declaration on its base and Radiant faces, its Activates' included. The client's
//! `modeText.test.ts` reads it to hold `MODE_TEXT` to every short Choose one a card offers, since the
//! words a choice card shows come from there and the options themselves live only in the scripts. The
//! export fails when the file differed from what the scripts make, having rewritten it: commit the
//! result. Its `"//"` key says where it comes from; its reader skips it.

use indexmap::IndexMap;
use jackioh_engine::{PromptKind, Script};
use serde::Serialize;

/// The file the client's coverage test reads, relative to this crate.
const OUTPUT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../apps/web/src/game/cardModes.json"
);

/// The file's `"//"` entry.
const HEADER: &str = "Generated from the card scripts by crates/tools/tests/export_card_modes.rs (#2646). Do not edit: run cargo test -p jackioh-tools --test export_card_modes.";

/// One card's declared Choose ones, each the list of its options, by face.
#[derive(Serialize)]
struct Faces {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    base: Vec<Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    radiant: Vec<Vec<String>>,
}

#[derive(Serialize)]
struct CardModes {
    #[serde(rename = "//")]
    header: &'static str,
    #[serde(flatten)]
    cards: IndexMap<String, Faces>,
}

/// The options of each `mode` declaration on one face: the card's own, then each Activate's.
fn declared(script: &Script) -> Vec<Vec<String>> {
    script
        .modes
        .iter()
        .chain(
            script
                .activations
                .iter()
                .flat_map(|activation| activation.modes.iter()),
        )
        .filter(|decl| decl.kind == PromptKind::Mode)
        .map(|decl| decl.options.clone())
        .collect()
}

fn render() -> String {
    let cards = jackioh_cards::scripts_of()
        .into_iter()
        .map(|(id, scripts)| {
            let faces = Faces {
                base: declared(&scripts.base),
                radiant: declared(&scripts.radiant),
            };
            (id, faces)
        })
        .filter(|(_, faces)| !faces.base.is_empty() || !faces.radiant.is_empty())
        .collect();
    let file = CardModes {
        header: HEADER,
        cards,
    };
    let text = serde_json::to_string_pretty(&file).unwrap_or_else(|error| panic!("cardModes.json: {error}"));
    format!("{text}\n")
}

#[test]
fn export_card_modes() {
    let text = render();
    let current = std::fs::read_to_string(OUTPUT).unwrap_or_default();
    if current != text {
        std::fs::write(OUTPUT, &text).unwrap_or_else(|error| panic!("{OUTPUT}: {error}"));
    }
    assert!(
        current == text,
        "{OUTPUT} differed from what the card scripts declare; this run rewrote it: commit the result"
    );
}
