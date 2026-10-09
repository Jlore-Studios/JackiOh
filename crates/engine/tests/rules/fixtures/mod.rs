//! `packages/engine/test/fixtures/*.ts`: the test-only card scripts and catalogs the engine tests
//! register through the testkit's thread-local override (SURFACE §8). Written once by part 1; the
//! files are part 24's.

pub mod activate;
pub mod board_history;
pub mod call_to_chaos_plus;
pub mod catalog;
pub mod combat;
pub mod copied_text;
pub mod core_patches;
pub mod credit;
pub mod damage_combat;
pub mod datacenter;
pub mod field;
pub mod fruit;
pub mod generation;
pub mod harness;
pub mod instance_data;
pub mod kill_credit;
pub mod ky_test;
pub mod last_boards;
pub mod papaya;
pub mod play_pipeline_a;
pub mod play_pipeline_b;
pub mod prompt_harness;
pub mod prompts;
pub mod quests;
pub mod rng_child;
pub mod scripts;
pub mod turn;
pub mod twice_forward;
pub mod validator_loadouts;
