//! The ported AI tests (SURFACE §1): one module per `packages/ai/test/<x>.test.ts`. Written once by
//! part 1; the files are part 17's.

pub mod activate;
pub mod alt_play;
pub mod answer_key;
pub mod decide;
pub mod deck;
pub mod determinize_shown_cost;
pub mod dev_run;
pub mod evaluate;
pub mod evaluate_v020;
pub mod lethal;
pub mod match_;
pub mod match_refusal;
pub mod observe;
pub mod observe_instance_data;
pub mod prompts_v020;
pub mod puzzles;
pub mod redact_announce;
pub mod redact_backrow_piles;
pub mod redact_board_history;
pub mod redact_fusion;
pub mod redact_last_boards;
pub mod redact_live_face_down;
pub mod redact_play_records;
pub mod redact_twice;
pub mod reply;
pub mod search;
pub mod shadow_ban;
pub mod support;
pub mod surface;
pub mod tutorial_tier;
