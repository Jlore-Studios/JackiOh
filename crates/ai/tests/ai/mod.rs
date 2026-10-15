//! The ported AI tests (SURFACE §1): one module per `packages/ai/test/<x>.test.ts`. Written once by
//! part 1; the files are part 17's. `lethal`, `match_`, `shadow_ban` and `tutorial_tier` are not
//! named here: `ai_slow_a.rs` and `ai_slow_b.rs` run them, each in a CI job of its own (#873).

pub mod activate;
pub mod alt_play;
pub mod answer_key;
pub mod decide;
pub mod deck;
pub mod determinize_shown_cost;
pub mod dev_run;
pub mod evaluate;
pub mod evaluate_v020;
pub mod match_refusal;
pub mod observe;
pub mod observe_instance_data;
pub mod position_model;
pub mod prompts_v020;
pub mod puzzles;
pub mod ranged_flying;
pub mod redact_announce;
pub mod redact_backrow_piles;
pub mod redact_board_history;
pub mod redact_custom;
pub mod redact_fusion;
pub mod redact_last_boards;
pub mod redact_live_face_down;
pub mod redact_play_records;
pub mod redact_twice;
pub mod replay_versions;
pub mod reply;
pub mod search;
pub mod secrets;
pub mod skill;
pub mod style;
pub mod support;
pub mod surface;
