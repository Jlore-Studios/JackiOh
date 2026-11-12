//! Stinkyscape #63 Miner v2 (SPEC §8.10 row 63). Unit 0/2 → 0/4, cost 1, Common.
//!   Base:    "Starship Piece, Equip
//!            Activate: Gain a random Stinkyscape Resource."
//!   Radiant: "Starship Piece, Equip
//!            Activate: Gain a random Radiant Stinkyscape Resource."
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it
//! fuses into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one
//! (R1852001); played alone, its death fuses a copy into its owner's Dormant Starship (R1852000).
//! R1872000: the Activate, once a turn (R384), gains one Resource rolled over every Resource the
//! catalog holds (R1840002) into the hand of the controller of the Unit it is on, a full hand burning
//! it (R2250000); the Radiant face's Resource is Radiant.

use jackioh_engine::effects::{GainResourcesArgs, gain_resources};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-063";

/// R1872000: "Gain a random [Radiant] Stinkyscape Resource".
fn mine(radiant: bool) -> Script {
    let label = if radiant {
        "Gain a random Radiant Stinkyscape Resource"
    } else {
        "Gain a random Stinkyscape Resource"
    };
    let dig = ActivationDecl {
        id: "mine".to_string(),
        label: label.to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(move |_ctx| {
            vec![gain_resources(GainResourcesArgs {
                count: 1,
                radiant: radiant.then_some(true),
                ..GainResourcesArgs::default()
            })]
        }),
    };
    Script {
        activations: vec![dig],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: mine(false),
        radiant: mine(true),
    }
}

// Stinkyscape #63 Miner v2 — SPEC §8.10 row 63, BUILD M12 row S 63 (R1840001, R1840002, R2250000).
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::stinkyscape_family as fam;

    #[test]
    fn r1872000_r2250000_its_activate_gains_one_random_resource_into_the_hand() {
        fam::miner_gains_one(ID, "s063");
    }

    #[test]
    fn r1840001_r2250000_into_a_full_hand_the_resource_burns() {
        fam::miner_burns_when_full(ID, "s063");
    }

    #[test]
    fn r384_once_a_turn_a_second_use_refused_and_again_next_turn() {
        fam::miner_uses_limit(ID, "s063", 1);
    }

    #[test]
    fn r1086_r1872000_equipped_onto_a_unit_the_host_takes_the_activate() {
        fam::miner_equip(ID, "s063");
    }

    #[test]
    fn radiant_the_resource_is_radiant() {
        fam::miner_radiant_gain(ID, "s063");
    }
}
