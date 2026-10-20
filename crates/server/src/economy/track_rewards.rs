//! A set track's reward table (SPEC §14.7, #1302): what reaching a level pays, kept as data.
//!
//! Each level pays once, as ledger rows, when its track reaches it. Packs are always the track's
//! own set; card tokens stand in for card rewards (a token unlocks any new card, so no card ids
//! are needed). The numbers live here, not in [`crate::config`]: the table is data, one per set,
//! and this is the first table of every set's, generous early (CLAUDE.md rule 9).

use serde::{Deserialize, Serialize};

/// What one set-track level pays when its track reaches it (SPEC §14.7).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LevelReward {
    /// Jlore granted with [`crate::economy::store::JloreReason::Track`].
    pub jlore: i64,
    /// Card packs of the track's own set, granted with
    /// [`crate::economy::store::EconomyReason::Track`].
    pub set_packs: i64,
    /// Card tokens (any new card), granted with [`crate::economy::store::EconomyReason::Track`].
    pub card_tokens: i64,
}

/// The reward one set-track level pays (SPEC §14.7): the table above, generous early. Past level
/// 10 every tenth level repeats level 10's, every fifth a pack, and any other a small Jlore purse.
pub fn reward_for_level(level: i64) -> LevelReward {
    match level {
        1 => LevelReward {
            jlore: 100,
            set_packs: 0,
            card_tokens: 0,
        },
        2 => LevelReward {
            jlore: 0,
            set_packs: 1,
            card_tokens: 0,
        },
        3 => LevelReward {
            jlore: 50,
            set_packs: 0,
            card_tokens: 0,
        },
        4 => LevelReward {
            jlore: 0,
            set_packs: 1,
            card_tokens: 0,
        },
        5 => LevelReward {
            jlore: 50,
            set_packs: 1,
            card_tokens: 0,
        },
        6 => LevelReward {
            jlore: 50,
            set_packs: 0,
            card_tokens: 0,
        },
        7 => LevelReward {
            jlore: 0,
            set_packs: 1,
            card_tokens: 0,
        },
        8 => LevelReward {
            jlore: 50,
            set_packs: 0,
            card_tokens: 0,
        },
        9 => LevelReward {
            jlore: 0,
            set_packs: 1,
            card_tokens: 0,
        },
        10 => LevelReward {
            jlore: 100,
            set_packs: 1,
            card_tokens: 1,
        },
        _ if level % 10 == 0 => LevelReward {
            jlore: 100,
            set_packs: 1,
            card_tokens: 1,
        },
        _ if level % 5 == 0 => LevelReward {
            jlore: 0,
            set_packs: 1,
            card_tokens: 0,
        },
        _ => LevelReward {
            jlore: 25,
            set_packs: 0,
            card_tokens: 0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn early_levels_are_generous() {
        assert_eq!(
            reward_for_level(1),
            LevelReward {
                jlore: 100,
                set_packs: 0,
                card_tokens: 0,
            }
        );
        assert_eq!(
            reward_for_level(2),
            LevelReward {
                jlore: 0,
                set_packs: 1,
                card_tokens: 0,
            }
        );
        assert_eq!(
            reward_for_level(5),
            LevelReward {
                jlore: 50,
                set_packs: 1,
                card_tokens: 0,
            }
        );
        assert_eq!(
            reward_for_level(10),
            LevelReward {
                jlore: 100,
                set_packs: 1,
                card_tokens: 1,
            }
        );
    }

    #[test]
    fn past_ten_tenths_repeat_ten_fifths_a_pack_the_rest_a_purse() {
        assert_eq!(reward_for_level(20), reward_for_level(10));
        assert_eq!(
            reward_for_level(15),
            LevelReward {
                jlore: 0,
                set_packs: 1,
                card_tokens: 0,
            }
        );
        assert_eq!(
            reward_for_level(11),
            LevelReward {
                jlore: 25,
                set_packs: 0,
                card_tokens: 0,
            }
        );
    }
}
