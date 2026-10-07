//! A card's running face and what it says about the card (docs/classic-sets.md B2.7, SPEC §5.2).
//!
//! B2.7: a face may carry its own type (Classic+ #22 Blood Moon's Radiant face is a Field Trap), so a
//! card's type is the running face's, and every rule that asks what an instance *is* reads it here.
//! A definition read on its own — a pool, a catalog filter — reads `def.type`, the base face's.
//!
//! Port of `packages/engine/src/faces.ts`. TS took `Pick<CardInstance, "defId" | "radiant">` (plus an
//! optional `zone`); the instance forms take a `CardInstance`, and `running_face_of` /
//! `card_type_of_face` take the definition id and face for a caller that has no instance (TS's
//! `{ defId, radiant }` literals, which carry no zone).

use crate::catalog::def_of;
use crate::state::{CardInstance, GameState};
use crate::wire::{CardFace, CardType, Row, Zone};

/// §5.2: the face the instance wears, radiant when it is.
pub fn running_face<'a>(state: &'a GameState, instance: &CardInstance) -> &'a CardFace {
    running_face_of(state, &instance.def_id, instance.radiant)
}

/// `running_face` for a definition and a face named directly (TS's `{ defId, radiant }` literal).
pub fn running_face_of<'a>(state: &'a GameState, def_id: &str, radiant: bool) -> &'a CardFace {
    let def = def_of(Some(state), def_id);
    if radiant { &def.radiant } else { &def.base }
}

/// B2.7: the card's type now — its running face's own type, else its definition's.
///
/// R383 (B3.1 rule 3): a card standing in a unit zone is a Unit for every rule, so an animated Field
/// Spell, Trap or Field Trap answers "Unit" while it stands there. Its trigger text still works the
/// way its face's type says — an animated Field Trap keeps firing as a trap — which is why the trap
/// machinery reads the face's own type (`animated::face_type_of`), never this.
pub fn card_type_of(state: &GameState, instance: &CardInstance) -> CardType {
    if let Zone::Field { row: Row::Units, .. } = instance.zone {
        return CardType::Unit;
    }
    card_type_of_face(state, &instance.def_id, instance.radiant)
}

/// `card_type_of` for a definition and a face with no zone (TS's `{ defId, radiant }` literal): the
/// running face's own type, else the definition's.
pub fn card_type_of_face(state: &GameState, def_id: &str, radiant: bool) -> CardType {
    let def = def_of(Some(state), def_id);
    let face = if radiant { &def.radiant } else { &def.base };
    face.type_.unwrap_or(def.type_)
}
