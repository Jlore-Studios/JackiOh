//! Port of `packages/engine/test/config.test.ts`: the BUILD §2 constants and the SPEC §11 "decide"
//! rows, at the values `packages/engine/src/config.ts` states (CLAUDE.md rule 9).

use jackioh_engine::testkit::*;

/// `describe("config constants (BUILD §2)")`.
mod config_constants_build_s2 {
    use super::*;

    #[test]
    fn matches_the_build_s2_table() {
        assert_eq!(DECK_SIZE, 20);
        assert_eq!(MAX_COPIES, 1);
        assert_eq!(MAX_MANA, 4);
        assert_eq!(HERO_HEALTH, 30);
        assert_eq!(OPENING_DRAW.to_vec(), vec![3, 4]);
        assert_eq!(OPENING_COINS.to_vec(), vec![0, 1]);
        assert_eq!(COIN_DEF_ID, "core-t-coin");
        assert_eq!(UNIT_ZONES, 5);
        assert_eq!(BACKROW_ZONES, 5);
        assert_eq!(GLITCH_NUMBERS.to_vec(), vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]); // C #18 Glitch in the System
        assert_eq!(DRAW_OFFERS_PER_TURN, 1);
        assert_eq!(DRAW_OFFER_BLOCK_TURNS, 3);
        assert_eq!(CALL_TO_CHAOS_CHAIN_CAP, 20);
        assert_eq!(CAST_ON_DRAW_CHAIN_CAP, 20);
        assert_eq!(ANTI_ONESHOT_CAP, ByFace { base: 5, radiant: 3 });
        assert_eq!(FUSE_COST_CAP, 4);
        assert_eq!(LIBRARY_CAP, 60);
        assert_eq!(MULLIGAN_ORDER, "draw-then-shuffle");
        assert_eq!(AI_END_TURN_PROBABILITY, 0.1);
    }

    #[test]
    fn holds_the_spec_s11_decide_rows_at_their_recommended_values() {
        const { assert!(CRY_ON_PLAY_ONLY) }; // R1
        assert_eq!(TURN_CAP_PLAYER_TURNS, 60); // R2, R389 (patch v0.2.0 doubled it)
        assert_eq!(HAND_CAP, 10); // R4
        const { assert!(!LANE_RESTRICTED_ATTACKS) }; // R5
        assert_eq!(ROTATION_RING, "two-rings"); // R14
        assert_eq!(GENN_GREED_EXILES, "odd"); // R26
        assert_eq!(FIENDER_STATS_MODE, "printed-plus-sum"); // R39
    }

    #[test]
    fn r3_fatigue_deals_n_on_the_nth_empty_draw() {
        assert_eq!([1, 2, 3].map(FATIGUE_DAMAGE).to_vec(), vec![1, 2, 3]);
    }

    #[test]
    fn r21_r346_r636_r49_random_keyword_pool_has_the_fourteen_listed_keywords_pierce_windfury_and_deft_last()
    {
        assert_eq!(
            RANDOM_KEYWORD_POOL.to_vec(),
            vec![
                "Taunt",
                "Armor 1",
                "Rush",
                "Charge",
                "First Strike",
                "Poisonous",
                "Lifesteal",
                "Reborn",
                "Divine Shield",
                "Trample",
                "Cleave",
                "Pierce",
                "Windfury",
                "Deft",
            ]
        );
    }

    #[test]
    fn r25_fib_index_clamps_at_11_89() {
        assert_eq!(
            [0, 1, 2, 3, 4, 5, 11].map(fib).to_vec(),
            vec![0, 1, 1, 2, 3, 5, 89]
        );
        assert_eq!(fib(12), 89);
        assert_eq!(fib(40), 89);
        assert_eq!(fib(-1), 0);
    }
}
