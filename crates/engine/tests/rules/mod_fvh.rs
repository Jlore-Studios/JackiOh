//! The engine's rule tests for Felinors vs Humans, in the `rules_fvh` binary (`tests/rules_fvh.rs`):
//! E1 #5236's keywords (`fvh_durability`, `fvh_keywords`), E2 #5237's lasting effects (`fvh_lasting`),
//! E3 #5238's verbs (`fvh_verbs`), E4 #5239's discovery (`fvh_discovery`) and F #5235's set (`fvh_set`).
//! It takes `fixtures` as the other rules binaries do, so a module reaches it as `crate::rules::fixtures`.

#[expect(
    dead_code,
    reason = "a fixture only another binary's modules use is dead code in this one"
)]
pub mod fixtures;
pub mod fvh_discovery;
pub mod fvh_durability;
pub mod fvh_keywords;
pub mod fvh_lasting;
pub mod fvh_set;
pub mod fvh_verbs;
