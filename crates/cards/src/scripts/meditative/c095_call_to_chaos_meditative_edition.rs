//! M #95 Call to Chaos (Meditative Edition) (SPEC §8.8 row 95, R28, R87, R423, R436, R1240–R1244). (4)
//! Spell, Call to Chaos, Legendary.
//!   Base:    "One random effect: Fuse your hand into one card and add 2 copies of it to your hand, all
//!            three of which cost (0); add 3 random CN cards to your hand, which cost (0); add 2 random
//!            Prime cards to your hand, which cost (0); your hero gains 8 Armor and you heal it 8; summon a
//!            Jade Beauty; summon 3 random Acclaimed cards; Bounce every enemy permanent, then Nerf each
//!            card bounced; summon a CN Golem; for the rest of the game, at the start of each of your
//!            turns, cast a random Call to Chaos; cast a random Call to Chaos."
//!   Radiant: "Three different random effects, resolved in the order listed: …" (the same ten).
//!
//! Core #95's subsystem (`subsystems::call_to_chaos`) with this edition's table (`CHAOS_MED_EFFECTS`,
//! `subsystems/call_to_chaos_meditative.rs`): the roll, the announcement both players read (R436), the
//! chain cap counting casts of every edition (R28, R1240) and the order the Radiant's three resolve in
//! (R423) are the one rule the three editions share. The face is passed explicitly, as C+ #73's file does.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-095";

pub fn script() -> CardScripts {
    // §8.8: "One random effect" of the ten.
    let base = Script {
        cry: Some(hook(|_ctx| {
            vec![subsystems::call_to_chaos(subsystems::CallToChaosArgs {
                radiant: Some(false),
                table: Some(subsystems::CHAOS_MED_EFFECTS),
            })]
        })),
        ..Script::default()
    };

    // §8.8, R423: three different random effects of the ten, resolved in the order the list writes them.
    let radiant = Script {
        cry: Some(hook(|_ctx| {
            vec![subsystems::call_to_chaos(subsystems::CallToChaosArgs {
                radiant: Some(true),
                table: Some(subsystems::CHAOS_MED_EFFECTS),
            })]
        })),
        ..Script::default()
    };

    CardScripts { base, radiant }
}
