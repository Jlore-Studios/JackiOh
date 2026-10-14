//! #90.1 CN-Virus (SPEC §8.5, §7, §4.4, R57, R58, R70, R80, R316, R350).
//!
//! Base: "Pierce. Cast on draw: take 1 damage; at end of turn, shuffle 2 copies of this into your deck".
//! Radiant: take 2 damage, 3 copies (§8's cell, R275: both numbers scale).
//!
//! A spell token (§7, R11): `static_flags.cast_on_draw` is the whole of "Cast on draw", on both faces.
//! The draw casts it free as a card played (R70) and caps the chain (R58). It hits the drawer's own
//! hero (control follows ownership off the field, R12) as damage, not "lose health" (R18): §4.4's
//! pipeline, its printed Pierce (patch v0.3.9) skipping the hero's Armor, and a hit reduced to 0 triggers nothing (R63) but the copies are owed.
//! The copies wait for the end of the turn it was cast on, whoever's (R350), at §2.2's delayed-effect
//! point in R68's order, so a chain casts only library viruses; a cast after those have begun is due
//! at the next such point for the same player (R62). They are fresh copies with the cast face's
//! radiant flag (R57); a full library refuses (R80, R316).


pub const ID: &str = "core-090-1";

// No `mod tests` here: #90.1's behaviour is proved by #90 CN Viral Injection's tests
// (`c090_cn_viral_injection.rs`).
