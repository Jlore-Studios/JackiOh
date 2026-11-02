//! C+ #44 Simplicity Audit and #45 Complexity Audit (SPEC §8.7 rows 44, 45; docs/classic-sets.md E36):
//! the permanents whose card has fewer (or more) lines of code than the Audit's own. `loc` is frozen
//! card data in `catalog.json` (E36, SURFACE §7.5) that nothing in Rust recomputes; a fused card's is
//! its ingredients' sum (R77). The cards keep their own wiring: their own `loc` is what they compare.

use serde::{Deserialize, Serialize};

use crate::state::{CardInstance, GameState};
use crate::wire::{PlayerId, Row, opponent_of};

/// E36: a definition's lines of code. A card with no script file yet has none, and reads 0.
pub fn lines_of_code(state: &GameState, def_id: &str) -> i32 {
    crate::catalog::def_of(Some(state), def_id).loc.unwrap_or(0)
}

/// Who runs the Audit, who is active (R68), the `loc` compared against, the comparison's direction and
/// whether only the opponent's side counts.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuditArgs {
    pub controller: PlayerId,
    pub active: PlayerId,
    pub loc: i32,
    pub more: bool,
    pub enemy_only: bool,
}

/// The permanents an Audit run by `controller` exiles now: the cards acting on the field (never one
/// dormant under a Stack, §3.2), `active`'s side first (R68; a preview never reads `state.active`,
/// R280) or the opponent's only, whose `loc` is strictly below `loc`, or above it when `more`.
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
                // R1081000: a Unit standing in the backrow is among the active units already; scan it once.
                let extra: Option<CardInstance> = crate::zones::card_at(state, slot)
                    .filter(|card| !cards.iter().any(|held| held.id == card.id))
                    .cloned();
                cards.extend(extra);
            }
            cards
        })
        .filter(|card| {
            let loc = lines_of_code(state, &card.def_id);
            if args.more { loc > args.loc } else { loc < args.loc }
        })
        .collect()
}
