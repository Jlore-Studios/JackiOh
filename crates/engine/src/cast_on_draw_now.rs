//! Whether a card is being cast on draw right now (SPEC §2.4, §6.2, R58; Classic+ #26 Tommy Tempo).
//!
//! "Cast on draw:" names what the card does when its draw casts it, and only then: a Tommy Tempo played
//! from a hand is a plain Unit and ends nothing. The draw holds its `drawn` back until that cast has
//! resolved (`draw_complete::hold_draw`), under the id the card was drawn as, which a Unit keeps through
//! the cast — so while the hold stands, the card is being cast on draw.
//!
//! Port of `packages/engine/src/castOnDrawNow.ts`.

use crate::state::{CardInstance, GameState};

/// R58: whether `card` is being cast by the draw that drew it.
pub fn is_cast_on_draw(state: &GameState, card: &CardInstance) -> bool {
    state
        .held_draws
        .as_ref()
        .is_some_and(|held| held.contains(&card.id))
}
