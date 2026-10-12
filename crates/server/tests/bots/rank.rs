//! R1452 (SPEC §9.12): the rank the match screen shows for a ladder bot's seat, the one its rating
//! calls for among the placed players' (R606), on its own tier's floor as a placed player's always
//! stands, never Jlorious, and never Mythic Grape unless more players stand in Mythic than Jlorious
//! holds (`JLORIOUS_SIZE`), since until then every one of them is Jlorious (R608).
//!
//! `jackioh_server::ranked::ladder::called_for_rank`.

use jackioh_server::ranked::ladder::{
    GrapeTier, VisibleRank, called_for_rank, percentile_of, place_of, target_ladder, tier_bottom,
};

#[test]
fn r1452_a_bots_rank_stands_on_its_own_tiers_floor() {
    let others: Vec<f64> = (0..40).map(|at| 900.0 + f64::from(at) * 5.0).collect();
    let mut tiers = Vec::new();
    for rating in [600.0, 950.0, 1000.0, 1050.0, 1100.0] {
        let VisibleRank::Grape { tier, floor, .. } = called_for_rank(rating, &others, false) else {
            panic!("a bot's rank is a Grape rank");
        };
        assert_eq!(floor, tier, "{rating}");
        tiers.push(tier);
    }
    tiers.dedup();
    assert!(
        tiers.len() > 1,
        "the ratings call for more than one tier: {tiers:?}"
    );
    // Nobody placed: the percentile is one half, and the rank its middle.
    assert!(matches!(
        called_for_rank(1000.0, &[], false),
        VisibleRank::Grape { .. }
    ));
}

#[test]
fn r1452_a_bot_shows_mythic_grape_only_once_a_player_holds_it() {
    // Rated above all 200 placed players: R606's target is in Mythic.
    let others: Vec<f64> = (0..200).map(|at| 800.0 + f64::from(at)).collect();
    let target = place_of(target_ladder(&percentile_of(1500.0, &others)));
    assert_eq!(target.tier, GrapeTier::Mythic, "the rating calls for Mythic");

    // No placed player past Jlorious's hundred: the top of the tier below.
    let below = place_of(tier_bottom(GrapeTier::Mythic as i32) - 1);
    let VisibleRank::Grape {
        tier,
        division,
        pips,
        floor,
        ..
    } = called_for_rank(1500.0, &others, false)
    else {
        panic!("a Grape rank");
    };
    assert_ne!(tier, GrapeTier::Mythic);
    assert_eq!(
        (tier, division, pips, floor),
        (below.tier, below.division, below.pips, below.tier)
    );

    // Players past Jlorious's hundred hold Mythic Grape, and so may the bot.
    let VisibleRank::Grape { tier, floor, .. } = called_for_rank(1500.0, &others, true) else {
        panic!("a Grape rank");
    };
    assert_eq!((tier, floor), (GrapeTier::Mythic, GrapeTier::Mythic));
}
