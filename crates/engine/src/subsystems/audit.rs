//! C+ #44 Simplicity Audit and #45 Complexity Audit (SPEC §8.7 rows 44, 45; docs/classic-sets.md E36):
//! the permanents whose card has fewer (or more) lines of code than the Audit's own. `loc` is card data
//! the catalog generator writes (E36); a fused card's is its ingredients' sum (R77, `subsystems::fuse`).
//! The cards keep their own wiring — the mode, the exile, the preview — because their own `loc` is
//! what they compare against, and a card file that only named this module would be three lines long.
//!
//! Port of `packages/engine/src/subsystems/audit.ts`. Since v0.3.0 `loc` is frozen data in
//! `catalog.json` (SURFACE §7.5): nothing here, or anywhere in Rust, recomputes it.

use serde::{Deserialize, Serialize};

use crate::state::{CardInstance, GameState};
use crate::wire::{PlayerId, Row, opponent_of};

/// E36: a definition's lines of code. A card with no script file yet has none, and reads 0.
pub fn lines_of_code(state: &GameState, def_id: &str) -> i32 {
    crate::catalog::def_of(Some(state), def_id).loc.unwrap_or(0)
}

/// `auditTargets`' argument (TS's inline object type): who runs the Audit, who is active (R68), the
/// `loc` compared against, the direction of the comparison and whether only the opponent's side counts.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuditArgs {
    pub controller: PlayerId,
    pub active: PlayerId,
    pub loc: i32,
    pub more: bool,
    pub enemy_only: bool,
}

/// The permanents an Audit run by `controller` exiles now: the cards acting on the field — tops of
/// piles (a Unit a carrier holds too) and backrow cards, face-down ones included, never a card dormant
/// under a Stack (§3.2) — on both sides, `active`'s first (R68; the caller says who is active, since a
/// preview never reads `state.active`, R280), or the opponent's only; each one whose card's `loc` is
/// strictly below `loc` (`more` false) or above it (`more` true). Equal stays.
pub fn audit_targets(state: &GameState, args: AuditArgs) -> Vec<CardInstance> {
    let enemy = opponent_of(args.controller);
    let sides = if args.enemy_only {
        vec![enemy]
    } else {
        vec![args.active, opponent_of(args.active)]
    };
    sides
        .into_iter()
        .flat_map(|player| {
            let mut cards = crate::zones::active_units_of(state, player)
                .into_iter()
                .cloned()
                .collect::<Vec<_>>();
            for slot in crate::zones::slots_of(player, Row::Backrow) {
                cards.extend(crate::zones::card_at(state, slot).cloned());
            }
            cards
        })
        .filter(|card| {
            let loc = lines_of_code(state, &card.def_id);
            if args.more { loc > args.loc } else { loc < args.loc }
        })
        .collect()
}
