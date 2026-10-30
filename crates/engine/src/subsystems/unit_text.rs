//! Copy a Unit's text (R1850000; Stinkyscape #20 Tnack Trav: "Copy the text of a Legendary or Mythic
//! Unit on the field", its Radiant face the Radiant text).
//!
//! The copy is a record in the copier's memory, `{ defId, radiant }` (`effects::copy_text`), which R78's
//! reset ends as the card leaves the field. Every reader of the text a card runs asks `text_def_of`:
//! `scripts::script_of`, `layers::printed_keywords_of`, `prompts`, `params`, `triggers` and
//! `subsystems::activate`. The rest (`layers::face_of`: name, cost, type, tags, rarity, printed stats)
//! and the Radiant flag are the copier's own.

use serde_json::Value;

use crate::state::{CardInstance, PlayRecord};

/// R1850000: where a copier keeps the text it copied.
pub const UNIT_TEXT_KEY: &str = "__unitText";

/// R1850000: the text this card copied from a Unit, if it copied one.
pub fn unit_text_of(card: &CardInstance) -> Option<PlayRecord> {
    card.memory.get(UNIT_TEXT_KEY)?;
    let (def_id, radiant) = text_def_of(card);
    Some(PlayRecord {
        def_id: def_id.to_string(),
        radiant,
    })
}

/// R1850000: the definition and face whose text this card runs — the copied Unit's for a copier, its
/// own otherwise. Borrowed, since the layers ask it on every read.
pub fn text_def_of(card: &CardInstance) -> (&str, bool) {
    if !card.memory.is_empty()
        && let Some(record) = card.memory.get(UNIT_TEXT_KEY).and_then(Value::as_object)
        && let Some(def_id) = record.get("defId").and_then(Value::as_str)
    {
        return (
            def_id,
            record.get("radiant").and_then(Value::as_bool) == Some(true),
        );
    }
    (card.def_id.as_str(), card.radiant)
}
