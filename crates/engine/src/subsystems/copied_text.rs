//! Copy the last Spell's text (docs/classic-sets.md B5 E14; Classic #57 Echo; SPEC §8.6 row 57, R399,
//! R545–R547): a card whose running face sets `staticFlags.copiesLastSpell` HAS the text of
//! `state.lastSpell`, the record every Spell play and cast writes at §10.5 step 4 (never a countered
//! play, B5 E1, nor a copier, whose `recordsPlayAs` names what it copied, so two copiers never loop, E4).
//!
//! In a hand or a deck the copy follows the record live. As the card is played (§10.5 step 1) or cast
//! it is fixed, held in its memory from the resolving zone on (R546), so its declarations (R81), Echo
//! repeats and prompt continuations keep that face until R215's reset as it lands. Declarations, target
//! checks, `preview` (R280) and `conditionMet` (R195) read `text_face_of`; the view shows it (R243).
//! X (R545): the copier pays its own price and a copied X-cost text chooses X (B5 E12). Text answering
//! from a pile the card has landed in (§5.1's return, #23, #24, #31) is not had (R547). Everything else
//! (name, Cost, type, tags, "this") is the copier's own; a fused card is never a copier (R77).

use serde_json::{Value, json};

use crate::script::{HookArgs, StaticFlags};
use crate::scripts::ScriptRef;
use crate::state::{CardInstance, GameState, PlayRecord};
use crate::wire::CardCost;

/// Where a copier being played keeps the copy it resolves (R546): its memory, written as it enters
/// the resolving zone, reset by R215 as it leaves. `null` is "fixed to nothing" (no Spell had been
/// played as the play began), which is not the same as absent.
pub const COPIED_TEXT_KEY: &str = "__copiedText";

/// An unfused card's flags on the face it wears, none for a Vanilla: `copies_text` has no state in
/// hand and a fused id never reaches it, so the registry alone answers.
fn registry_flags_of(card: &CardInstance) -> StaticFlags {
    if card.vanilla {
        return StaticFlags::default();
    }
    crate::scripts::registered_entry(card.def_id.as_str())
        .and_then(|entry| {
            let face = if card.radiant { &entry.radiant } else { &entry.base };
            face.static_flags.clone()
        })
        .unwrap_or_default()
}

/// The running face's script (§5.2), none for a Vanilla (§6.3, R115); `scripts::script_of` composes a
/// fused card's scripts on lookup (SURFACE §6.6).
fn face_script_of(state: &GameState, card: &CardInstance) -> ScriptRef {
    crate::scripts::script_of(state, card)
}

/// Whether this card's running face copies the last Spell's text (`staticFlags.copiesLastSpell`).
pub fn copies_text(card: &CardInstance) -> bool {
    if crate::catalog::fused_id_parts(None, &card.def_id).is_some() {
        return false;
    }
    registry_flags_of(card).copies_last_spell == Some(true)
}

/// `null` stays "fixed to nothing" (`Some(None)`), a record with a string `defId` is the copy
/// (`Some(Some(…))`), and anything else is as good as absent (`None`).
fn record_from(raw: &Value) -> Option<Option<PlayRecord>> {
    if raw.is_null() {
        return Some(None);
    }
    let record = raw.as_object()?;
    let def_id = record.get("defId")?.as_str()?;
    Some(Some(PlayRecord {
        def_id: def_id.to_string(),
        radiant: record.get("radiant") == Some(&Value::Bool(true)),
    }))
}

/// The copy fixed on a copier being played, `Some(None)` for one fixed to nothing, `None` for none.
fn fixed_copy_of(card: &CardInstance) -> Option<Option<PlayRecord>> {
    let raw = card.memory.get(COPIED_TEXT_KEY)?;
    record_from(raw)
}

/// R399, R546: the Spell whose text this card has now — the copy fixed on it while it is played, else
/// the last Spell either player played — or null: not a copier, or nothing to copy.
pub fn copied_text_of(state: &GameState, card: &CardInstance) -> Option<PlayRecord> {
    if !copies_text(card) {
        return None;
    }
    if let Some(fixed) = fixed_copy_of(card) {
        return fixed;
    }
    state.last_spell.as_ref().map(|last| PlayRecord {
        def_id: last.def_id.clone(),
        radiant: last.radiant,
    })
}

/// R546: fix the copy a copier resolves as its play or cast moves it into the resolving zone: `record`
/// when the play fixed it earlier (§10.5 step 1; `Some(None)` fixed to nothing), else the last Spell now.
pub fn fix_copied_text(state: &mut GameState, card_id: &str, record: Option<Option<PlayRecord>>) {
    let Some(card) = crate::state::find_instance(state, card_id) else {
        return;
    };
    if !copies_text(card) {
        return;
    }
    let copy = match record {
        None => copied_text_of(state, card),
        Some(given) => given,
    };
    let value = match copy {
        None => Value::Null,
        Some(copy) => json!({ "defId": copy.def_id, "radiant": copy.radiant }),
    };
    if let Some(card) = crate::state::find_instance_mut(state, card_id) {
        card.memory.insert(COPIED_TEXT_KEY.to_string(), value);
    }
}

/// R399, R546: the card as its text runs — for a copier with a copy, its own instance under the copied
/// definition and face, which every reader of a declaration, a target check, a `preview` or a
/// `conditionMet` is handed; every other card is itself. A read: the copier's instance is not changed.
pub fn text_face_of(state: &GameState, card: &CardInstance) -> CardInstance {
    match copied_text_of(state, card) {
        None => card.clone(),
        Some(copy) => CardInstance {
            def_id: copy.def_id,
            radiant: copy.radiant,
            ..card.clone()
        },
    }
}

/// The script a card's text runs now: the copied face's for a copier with a copy, its own otherwise.
pub fn running_script_of(state: &GameState, card: &CardInstance) -> ScriptRef {
    match copied_text_of(state, card) {
        None => face_script_of(state, card),
        Some(copy) => crate::scripts::face_ref(state, &copy.def_id, copy.radiant),
    }
}

/// R546: the Echo X the copied face prints, which adds to the copier's own Echo (0 for any other card).
/// R802: a copied face that computes its Echo X prints the larger of that and its fixed one, computed
/// on the copier's instance under the copied face, as every reader of the copied text is handed it.
pub fn copied_echo(state: &GameState, card: &CardInstance) -> i32 {
    if copied_text_of(state, card).is_none() {
        return 0;
    }
    let script = running_script_of(state, card);
    let echo = script
        .static_flags
        .as_ref()
        .and_then(|flags| flags.echo)
        .unwrap_or(0);
    let face = text_face_of(state, card);
    let computed = script.echo_x.as_ref().map_or(0, |hook| {
        hook(HookArgs {
            state,
            self_: &face,
            radiant: face.radiant,
        })
    });
    echo.max(computed).max(0)
}

/// R547: whether the copied face casts on draw (§2.4), for a copier being drawn.
pub fn copied_casts_on_draw(state: &GameState, card: &CardInstance) -> bool {
    if copied_text_of(state, card).is_none() {
        return false;
    }
    running_script_of(state, card)
        .static_flags
        .as_ref()
        .and_then(|flags| flags.cast_on_draw)
        == Some(true)
}

/// R545: whether a play of this copier chooses an X for the text it copies — an X-cost Spell's whose X
/// no `cost` hook fixes (§2.3, R43). The copier itself is never an X-cost card: it pays its own price.
pub fn copied_chooses_x(state: &GameState, card: &CardInstance) -> bool {
    let Some(copy) = copied_text_of(state, card) else {
        return false;
    };
    if crate::catalog::def_of(Some(state), &copy.def_id).cost != CardCost::X {
        return false;
    }
    running_script_of(state, card).cost.is_none()
}
