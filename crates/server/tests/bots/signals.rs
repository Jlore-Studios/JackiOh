//! R639000 (SPEC §9.12): a ladder bot's concessions, draw answers, idle turns, rematch plans and aim
//! leads, each drawn on its own stream from a fitted row (`bots::signals`). These are the pure
//! draws; `tests/actor/bot_seat.rs` and `tests/api/bots.rs` prove the actor and the rematch route
//! act on them.

use jackioh_engine::PlayerId;
use jackioh_server::bots::signals::{accepts_draw, aim_lead_ms, concedes, idles_turn, rematch_plan};
use jackioh_server::config::{
    AIM_RELAY_INTERVAL_MS, BOT_AIM_LEAD_MAX_MS, BOT_AIM_LEAD_MIN_MS, BOT_REMATCH_DELAY_MAX_MS,
    BOT_REMATCH_DELAY_MIN_MS, BOT_REMATCH_FIT, BOT_REMATCH_LINGER_MAX_MS, BOT_REMATCH_LINGER_MIN_MS,
    ConcedeFit, DrawAnswerFit, REMATCH_OFFER_TTL_MS, RematchFit, TimeoutFit,
};

fn concede_fit(share: f64, deficit_sigma: f64) -> ConcedeFit {
    ConcedeFit {
        samples: 0,
        share,
        min_turn: 6,
        deficit_mu: 40.0,
        deficit_sigma,
    }
}

/// How many of `trials` came out true.
fn share_of(trials: usize, mut trial: impl FnMut(usize) -> bool) -> f64 {
    (0..trials).filter(|at| trial(*at)).count() as f64 / trials as f64
}

#[test]
fn r639000_concede_draws_its_temper_once_and_waits_for_turn_and_deficit() {
    // Every bot a conceder, at exactly the mean deficit: the turn and the deficit decide alone.
    let sure = concede_fit(1.0, 0.0);
    for seed in ["temper-a", "temper-b", "temper-c"] {
        for home in [PlayerId::P1, PlayerId::P2] {
            assert!(!concedes(seed, home, 5, 1_000.0, &sure), "never before min_turn");
            assert!(
                !concedes(seed, home, 6, 39.9, &sure),
                "never short of the threshold"
            );
            assert!(concedes(seed, home, 6, 40.0, &sure));
            assert!(concedes(seed, home, 30, 41.0, &sure));
        }
    }
    // No conceder ever concedes, however far behind.
    let never = concede_fit(0.0, 20.0);
    assert!((0..200).all(|at| !concedes(&format!("never-{at}"), PlayerId::P2, 20, 1e9, &never)));

    // The temper is the match's: the same seed and seat answer the same on every turn past
    // `min_turn`, and once a deficit is enough every larger one is.
    let fit = concede_fit(0.5, 20.0);
    for at in 0..60 {
        let seed = format!("temper-{at}");
        let mut conceded = false;
        for deficit in (-40..=160).step_by(5).map(f64::from) {
            let now = concedes(&seed, PlayerId::P1, 6, deficit, &fit);
            assert_eq!(
                now,
                concedes(&seed, PlayerId::P1, 40, deficit, &fit),
                "{seed} at {deficit}"
            );
            assert!(now || !conceded, "{seed}: conceded below {deficit} but not at it");
            conceded = now;
        }
    }

    // The share of conceders is the fit's.
    let share = share_of(4_000, |at| {
        concedes(
            &format!("share-{at}"),
            PlayerId::P2,
            10,
            1e9,
            &concede_fit(0.4, 0.0),
        )
    });
    assert!((0.37..=0.43).contains(&share), "{share}");
}

#[test]
fn r639000_a_draw_is_accepted_only_behind_at_the_fitted_share() {
    let always = DrawAnswerFit {
        samples: 0,
        accept_share: 1.0,
    };
    let never = DrawAnswerFit {
        samples: 0,
        accept_share: 0.0,
    };
    for turn in 1..20 {
        assert!(
            !accepts_draw("draw", PlayerId::P1, turn, 0.0, &always),
            "level: declined"
        );
        assert!(
            !accepts_draw("draw", PlayerId::P1, turn, -15.0, &always),
            "ahead: declined"
        );
        assert!(accepts_draw("draw", PlayerId::P1, turn, 0.5, &always));
        assert!(!accepts_draw("draw", PlayerId::P1, turn, 50.0, &never));
    }
    let fit = DrawAnswerFit {
        samples: 0,
        accept_share: 0.25,
    };
    let share = share_of(4_000, |at| {
        accepts_draw(&format!("draw-{at}"), PlayerId::P2, 7, 10.0, &fit)
    });
    assert!((0.22..=0.28).contains(&share), "{share}");
    // One offer's answer is the turn's own: asked again, it answers the same.
    for at in 0..100 {
        let seed = format!("again-{at}");
        assert_eq!(
            accepts_draw(&seed, PlayerId::P2, 9, 10.0, &fit),
            accepts_draw(&seed, PlayerId::P2, 9, 10.0, &fit)
        );
    }
}

#[test]
fn r639000_an_idle_turn_follows_the_fitted_chance() {
    let at = |chance_per_turn| TimeoutFit {
        samples: 0,
        chance_per_turn,
    };
    assert!((1..200).all(|turn| !idles_turn("idle", PlayerId::P2, turn, &at(0.0))));
    assert!((1..200).all(|turn| idles_turn("idle", PlayerId::P2, turn, &at(1.0))));
    let share = share_of(10_000, |turn| {
        idles_turn(
            "idle-share",
            PlayerId::P1,
            i32::try_from(turn).unwrap_or(0),
            &at(0.01),
        )
    });
    assert!((0.005..=0.015).contains(&share), "{share}");
}

#[test]
fn r639000_a_rematch_plan_stays_inside_its_windows() {
    // The windows nest: an offer always goes inside the linger, and the linger ends long before an
    // offer would lapse.
    const { assert!(BOT_REMATCH_DELAY_MAX_MS < BOT_REMATCH_LINGER_MIN_MS) };
    const { assert!(BOT_REMATCH_LINGER_MAX_MS < REMATCH_OFFER_TTL_MS) };
    for at in 0..500 {
        let seed = format!("plan-{at}");
        for home in [PlayerId::P1, PlayerId::P2] {
            let plan = rematch_plan(&seed, home, &BOT_REMATCH_FIT);
            assert_eq!(
                plan,
                rematch_plan(&seed, home, &BOT_REMATCH_FIT),
                "drawn once, the same"
            );
            assert!((BOT_REMATCH_LINGER_MIN_MS..=BOT_REMATCH_LINGER_MAX_MS).contains(&plan.linger_ms));
            for delay in [plan.offer_after_ms, plan.reply_ms] {
                assert!((BOT_REMATCH_DELAY_MIN_MS..=BOT_REMATCH_DELAY_MAX_MS).contains(&delay));
                assert!(delay < plan.linger_ms);
            }
        }
    }
    let fit = |offer_share, accept_share| RematchFit {
        samples: 0,
        offer_share,
        accept_share,
    };
    for at in 0..100 {
        let seed = format!("plan-share-{at}");
        let none = rematch_plan(&seed, PlayerId::P2, &fit(0.0, 0.0));
        assert!(!none.offers && !none.accepts);
        let both = rematch_plan(&seed, PlayerId::P2, &fit(1.0, 1.0));
        assert!(both.offers && both.accepts);
    }
    let offers = share_of(4_000, |at| {
        rematch_plan(&format!("offers-{at}"), PlayerId::P2, &fit(0.15, 0.35)).offers
    });
    let accepts = share_of(4_000, |at| {
        rematch_plan(&format!("offers-{at}"), PlayerId::P2, &fit(0.15, 0.35)).accepts
    });
    assert!((0.12..=0.18).contains(&offers), "{offers}");
    assert!((0.32..=0.38).contains(&accepts), "{accepts}");
}

#[test]
fn r639000_aim_lead_stays_inside_its_window() {
    // Never so short the relay could not send the aim before the move.
    const { assert!(BOT_AIM_LEAD_MIN_MS >= AIM_RELAY_INTERVAL_MS) };
    let mut leads = Vec::new();
    for seq in 0..500 {
        let lead = aim_lead_ms("aim", PlayerId::P2, seq);
        assert_eq!(lead, aim_lead_ms("aim", PlayerId::P2, seq));
        assert!(
            (BOT_AIM_LEAD_MIN_MS..=BOT_AIM_LEAD_MAX_MS).contains(&lead),
            "{lead}"
        );
        leads.push(lead);
    }
    // Drawn, not fixed: the leads spread over the window.
    leads.sort_unstable();
    leads.dedup();
    assert!(leads.len() > 100, "{} distinct leads", leads.len());
}
