//! The number a formula comes to now (SPEC §10.8, §10.9, R280): the client may not do a card's
//! arithmetic (CLAUDE.md rule 7), so the script declares `preview`, a pure read, and `view_for` carries
//! its answer.
//!
//! It is asked only where the viewer may read the card (§10.8): their own hand in any phase (a value is
//! information, not R195's main-phase glow; §9.1), a unit on top of its pile (never one under a Stack,
//! R13), and a backrow card face-up or the viewer's own (R33). The hook runs as the card does (its face,
//! controller and zone) and reads what that controller may (§9.1), so a value's `ids` keep only the
//! cards the viewer may read where they lie (R1447). An empty answer is no preview: no key, not `[]`.

use crate::ownership::readers_where_it_lies;
use crate::script::{ConditionContext, ConditionZone};
use crate::state::{CardInstance, GameState, find_instance};
use crate::wire::{CardType, PlayerId, PreviewValue, Row, TallyView, Zone};

/// §10.8: "traps show as unknown, Field Spells are public": a backrow Trap or Field Trap is readable by
/// its current controller only until it flips face-up (R33). `view_for` and this module both read it,
/// so the two cannot drift apart.
pub fn backrow_is_public(state: &GameState, card: &CardInstance, viewer: PlayerId) -> bool {
    !is_face_down(state, card) || card.controller == viewer
}

/// R33, R371, R686: a backrow Trap or Field Trap neither flipped face-up nor revealed, so only its
/// controller may read it. `view_for` marks the controller's own view of one `unrevealed` from the same
/// answer, so the mark a client draws and the back the other player sees cannot disagree.
pub fn is_face_down(state: &GameState, card: &CardInstance) -> bool {
    let card_type = crate::faces::card_type_of(state, card);
    if card_type != CardType::Trap && card_type != CardType::FieldTrap {
        return false;
    }
    card.face_up != Some(true) && card.revealed != Some(true)
}

/// R1487000: whether a face-down card's lasting text (every text a reader takes off the board) sleeps
/// until it is revealed or face-up; its triggers and a self-Tribute stay live (R1149000). A
/// `Script::live_face_down` card (R767000) and the AI's def-less placeholder (R185) never sleep.
pub fn lasting_text_dormant(state: &GameState, card: &CardInstance) -> bool {
    crate::catalog::find_def(Some(state), &card.def_id).is_some()
        && is_face_down(state, card)
        && !crate::scripts::script_of(state, card).live_face_down
}

/// R1487000, R1839000: whether a card's lasting text is off, face-down (`lasting_text_dormant`) or
/// Dormant until it awakens (`slumber.rs`): what each board walk that reads lasting text asks.
pub fn lasting_text_off(state: &GameState, card: &CardInstance) -> bool {
    crate::slumber::is_slumbering(card) || lasting_text_dormant(state, card)
}

/// R1447, §10.8: the ids among `ids` that `viewer` may read where the card lies (R466's readers): a
/// face-down card only for its controller, a hand only for its holder, a library for nobody, and a
/// card that has ceased to exist never.
fn readable_ids(state: &GameState, ids: Vec<String>, viewer: PlayerId) -> Vec<String> {
    ids.into_iter()
        .filter(|id| {
            find_instance(state, id).is_some_and(|card| readers_where_it_lies(state, card).contains(&viewer))
        })
        .collect()
}

/// R280, §10.8: whether `viewer` may read `card` where the question places it.
fn may_preview(state: &GameState, card: &CardInstance, viewer: PlayerId, zone: ConditionZone) -> bool {
    if zone == ConditionZone::Hand {
        return matches!(card.zone, Zone::Hand { player } if player == viewer);
    }
    match card.zone {
        Zone::Field { row: Row::Units, .. } => !crate::zones::is_buried(state, card),
        Zone::Field { .. } => backrow_is_public(state, card, viewer),
        _ => false,
    }
}

/// R280: the labeled numbers `viewer` sees on `card`, or `None` for none. Pure: it copies what the
/// hook returns, so the view holds no reference into a script, and it calls the hook at most once.
pub fn preview_of(
    state: &GameState,
    card: &CardInstance,
    viewer: PlayerId,
    zone: ConditionZone,
) -> Option<Vec<PreviewValue>> {
    if !may_preview(state, card, viewer, zone) {
        return None;
    }
    // B5 E14, R547: a copier (Classic #57 Echo) previews the formula of the text it has, on that face.
    let script = crate::subsystems::copied_text::running_script_of(state, card);
    let hook = script.preview.as_ref()?;
    let face = crate::subsystems::copied_text::text_face_of(state, card);
    let values = hook(ConditionContext {
        state,
        self_: &face,
        controller: card.controller,
        radiant: face.radiant,
        zone,
        your_turn: state.active == card.controller,
    });
    // R372: a value the text names by a word (#93's grade letter) carries it as `display`; a value that
    // counts a set of cards (Classic+ #44, #45's Radiant "highlight targets") carries their ids.
    let copied: Vec<PreviewValue> = values
        .into_iter()
        .filter(|entry| !entry.label.is_empty())
        .map(|entry| PreviewValue {
            label: entry.label,
            value: entry.value,
            display: entry.display.filter(|display| !display.is_empty()),
            ids: entry.ids.map(|ids| readable_ids(state, ids, viewer)),
        })
        .collect();
    if copied.is_empty() { None } else { Some(copied) }
}

/// R2321000, §10.8: the counts the cards acting on the field keep on `unit` (`Script.tallies`), as
/// `viewer` sees them: each from a card the viewer may read where it lies (`may_preview`), asked as its
/// own preview is, in R68's order.
pub fn tallies_on(state: &GameState, unit: &CardInstance, viewer: PlayerId) -> Option<Vec<TallyView>> {
    let holders = crate::triggers::field_holders_where(state, |card| {
        crate::subsystems::copied_text::running_script_of(state, card)
            .tallies
            .is_some()
            && may_preview(state, card, viewer, ConditionZone::Field)
    });
    let tallies: Vec<TallyView> = holders
        .iter()
        .flat_map(|holder| {
            let card = &holder.card;
            let script = crate::subsystems::copied_text::running_script_of(state, card);
            let face = crate::subsystems::copied_text::text_face_of(state, card);
            let counts = script.tallies.as_ref().map_or_else(Vec::new, |hook| {
                hook(ConditionContext {
                    state,
                    self_: &face,
                    controller: card.controller,
                    radiant: face.radiant,
                    zone: ConditionZone::Field,
                    your_turn: state.active == card.controller,
                })
            });
            counts
                .into_iter()
                .filter(|tally| tally.instance_id == unit.id)
                .map(|tally| TallyView {
                    def_id: card.def_id.clone(),
                    value: tally.value,
                    goal: tally.goal,
                })
                .collect::<Vec<_>>()
        })
        .collect();
    if tallies.is_empty() { None } else { Some(tallies) }
}

/// R2476000: what a value that names something prints before there is anything to name (R982's dash):
/// a Carnivorous Cube that has eaten nothing, a Ticketed holding nothing.
pub const NOTHING_REMEMBERED: &str = "—";

/// R2476000: a value that names a card a card remembers, by its name as its face prints it, "Radiant "
/// before a Radiant one (`display`, R372), and 1; or, remembering none, the dash and 0. The name is
/// the catalog's (a card shown in Chinese reads it as every engine-made line does).
pub fn card_preview(state: &GameState, label: &str, named: Option<(&str, bool)>) -> PreviewValue {
    let display = named
        .and_then(|(def_id, radiant)| {
            crate::catalog::find_def(Some(state), def_id).map(|def| {
                if radiant {
                    format!("Radiant {}", def.name)
                } else {
                    def.name.clone()
                }
            })
        })
        .unwrap_or_else(|| NOTHING_REMEMBERED.to_string());
    PreviewValue {
        label: label.to_string(),
        value: i32::from(display != NOTHING_REMEMBERED),
        display: Some(display),
        ids: None,
    }
}

/// R2476000: a value that counts what a card remembers, a number printed as it is.
pub fn count_preview(label: &str, value: i32) -> PreviewValue {
    PreviewValue {
        label: label.to_string(),
        value,
        display: None,
        ids: None,
    }
}
