//! Copy the last Spell's text (docs/classic-sets.md B5 E14; Classic #57 Echo; SPEC §8.6 row 57, R399,
//! R545–R547).
//!
//! "This has the text of the last Spell either player played." The record is E4's (`playCounts.ts`):
//! `state.lastSpell = { defId, radiant }`, written at §10.5 step 4 by every Spell play and cast, never
//! by a countered play (B5 E1), and never by a copier itself — its `recordsPlayAs` names the Spell it
//! copied, or nothing when it copied nothing, so two copiers never loop (B5 E4). This module is the
//! other half: a card whose running face sets `staticFlags.copiesLastSpell` HAS that Spell's text.
//!
//! What "has the text" reaches, and where (R399, R546, R547):
//!   * In a hand or a deck the copy follows `state.lastSpell` live (`copiedTextOf`).
//!   * As the card is played (§10.5 step 1) or cast, the copy it will resolve is fixed: a play keeps it
//!     on its run until the announce moves the card into the resolving zone, and from there on the
//!     card's own memory holds it (`fixCopiedText`), so its declared choices, its resolution, its Echo
//!     repeats and every prompt continuation of the copied text keep that face even when a cast inside
//!     the resolution records a newer Spell. The memory goes with R215's reset as the card lands.
//!   * Declarations (targets, modes, R81) and the declaring card's own target checks are read off the
//!     copied face: `textFaceOf` returns the card as its text runs — the copier's instance under the
//!     copied definition and face — and `playChoices.resolvingFace` hands every declaration reader that
//!     card. Its instance id is the copier's, so a hand pick never offers the card being played.
//!   * The resolution runs the copied face's `cry` with the copier as `self` (`playSteps`' resolve and
//!     Echo steps pass `textFaceOf` to `prompts.runHookResumable`), so the continuation a prompt in it
//!     parks names the copied definition (`prompts.resumeSelf` reads the running `ctx.defId` first),
//!     and `param(ctx, key)` reads the copied definition's declared numbers on the copier's instance.
//!   * X (R545): the copier is played for its own printed cost; a copied X-cost text makes the play
//!     choose X, from 1 up to the mana left once that price is paid (`copiedChoosesX`,
//!     `playChoices.legalXValues`), and a cast of it asks its caster for X as any cast X card's does
//!     (B5 E12). An embiggen text resolves at its base price: no embiggen price is paid for the copier.
//!   * Echo (R546): the copied face's printed Echo X adds to the copier's own (`copiedEcho`).
//!   * Cast on draw (R547): a copier drawn while the last Spell casts on draw is cast (`draw.castsOnDraw`).
//!   * `preview` (R280) and `conditionMet` (R195) of the copied face answer for the copier in hand.
//!   * The view (R243, R399): the owner's hand view of a copier carries the copied face as `copies`.
//!     Text that answers from a pile the card has landed in — §5.1's end-of-turn return (#23, #24, #31) —
//!     is not had (R547): step 7 writes that flag off the card's own face, and the copy ends as it lands.
//!
//! Everything else about the card is its own: its name, (1) Cost, type and tags (a copied Book does
//! not make it a Book), its own Echo 1 on the Radiant face, and "this" in the copied text, which is the
//! copier — the card that resolves, lands and is counted. A fused card is never a copier: a Fuse joins
//! printed scripts (R77), and a copier prints none of the text it copies.
//!
//! Port of `packages/engine/src/subsystems/copiedText.ts`. TS's "the copy fixed on it" has three
//! states — absent, `null` ("fixed to nothing") and a record — so the private reader answers
//! `Option<Option<PlayRecord>>` (outer `None` is absent), and `fix_copied_text`'s `record?` argument
//! takes the same shape.

use serde_json::{Value, json};

use crate::script::StaticFlags;
use crate::scripts::ScriptRef;
use crate::state::{CardInstance, GameState, PlayRecord};
use crate::wire::CardCost;

/// Where a copier being played keeps the copy it resolves (R546): its memory, written as it enters
/// the resolving zone, JSON like all memory, reset by R215 as it leaves that zone. `null` is "fixed to
/// nothing" — no Spell had been played as the play began — which is not the same as absent.
pub const COPIED_TEXT_KEY: &str = "__copiedText";

/// TS `flagsOf(card)` for a card that is not fused: the registry's entry for its definition, on the
/// face it wears, or nothing for a Vanilla (`scripts.scriptOf`'s guard). `copiesText` asks it before
/// any state is in hand, and a fused id never reaches it, so the registry alone answers.
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

/// TS `scripts.scriptOf(card)`: the face that is running, radiant text once the instance is Radiant
/// (§5.2), and no script at all for a Vanilla instance (§6.3, R115). Read through
/// `scripts::script_of(state, def_id)`, which composes a fused card's scripts on lookup (SURFACE §6.6).
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

/// TS `recordFrom(raw)`: `null` stays "fixed to nothing" (`Some(None)`), a record with a string
/// `defId` is the copy (`Some(Some(…))`), and anything else is as good as absent (`None`).
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

/// R546: fix the copy a copier resolves, as its play or cast moves it into the resolving zone —
/// `record` when the play fixed it earlier (§10.5 step 1), else the last Spell now. Nothing for a card
/// that copies nothing.
///
/// The card is named by id and written in place (TS wrote through the live instance it was handed);
/// `record` is TS's `record?: PlayRecord | null`: `None` absent, `Some(None)` fixed to nothing.
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
pub fn copied_echo(state: &GameState, card: &CardInstance) -> i32 {
    if copied_text_of(state, card).is_none() {
        return 0;
    }
    let echo = running_script_of(state, card)
        .static_flags
        .as_ref()
        .and_then(|flags| flags.echo)
        .unwrap_or(0);
    echo.max(0)
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
