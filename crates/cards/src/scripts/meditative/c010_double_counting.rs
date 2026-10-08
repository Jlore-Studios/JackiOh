//! Meditative #10 Double Counting (SPEC §8.8 row 10; docs/meditative-set.md M6 #10; ME-TRIG (b); R820,
//! R822, R823). (2) Field Spell, Epic.
//!   Base:    "Aura: Your Cry and Death effects trigger {extra|additional time|additional times}." (extra 1)
//!   Radiant: the same, extra 2.
//!
//! The engine's Cry and Death multiplier (`Script.cry_death_extra`), Hearthstone's Brann Bronzebeard and
//! Baron Rivendare in one card: while this acts on its controller's field, the Cry of a permanent they
//! play or cast (§10.5 step 5) or trigger (C #54 Rewind) runs extra more times with the same choices,
//! and the Death of a card that died under their control runs extra more times on its snapshot (§4.5
//! step 3). A Spell's resolution is no Cry (R822). Several multipliers do not add: the highest holds
//! (R820). Both faces run one script; `param` reads the face's `extra`.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-010";

pub fn script() -> CardScripts {
    let base = Script {
        cry_death_extra: Some(read_hook(|args| param(&args, "extra"))),
        ..Script::default()
    };
    CardScripts {
        radiant: base.clone(),
        base,
    }
}
