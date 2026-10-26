//! What the AI may know of issue #1845's two engine primitives (SPEC §9.9, R185): a hidden Nerf is a
//! count no seat reads, its own hand's included (R1845000), so `redact` drops it from every card and a
//! decision never hangs on it; and a trade offer's two prompts are ordinary ones, which the AI answers
//! on either side of the table (R1845002).

use jackioh_ai::*;
use jackioh_engine::effects;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, ai_options, clone, every_card, is_legal};

fn p1_side() -> Value {
    json!({ "hand": ["core-008", "core-035"], "field": ["core-011"], "library": ["core-020", "core-053"] })
}

fn p2_side() -> Value {
    json!({ "hand": ["core-002", "core-011"], "field": ["core-019"], "library": ["core-005", "core-016"] })
}

fn base(seed: &str) -> GameState {
    jackioh_cards::register_all();
    scenario(json!({ "seed": seed, "p1": p1_side(), "p2": p2_side() }))
        .state()
        .clone()
}

mod r1845000_the_ai_reads_no_hidden_nerf {
    use super::*;

    #[test]
    fn r1845000_redact_forgets_every_hidden_nerf_the_seats_own_hand_included() {
        let base = base("hidden-nerfs-redact");
        let mut changed = clone(&base);
        for player in PLAYER_IDS {
            let side = &mut changed.players[player];
            for card in side
                .hand
                .iter_mut()
                .chain(side.library.iter_mut())
                .chain(side.units.iter_mut().flatten().flatten())
            {
                card.hidden_nerfs = Some(3);
            }
        }
        let public = redact(&changed, AI);
        assert!(every_card(&public).iter().all(|card| card.hidden_nerfs.is_none()));
        assert_eq!(hash_state(&public), hash_state(&redact(&base, AI)));
    }

    #[test]
    fn r1845000_decide_is_blind_to_the_seats_own_hidden_nerfs() {
        let base = base("hidden-nerfs-decide");
        let mut changed = clone(&base);
        for card in changed.players[AI].hand.iter_mut() {
            card.hidden_nerfs = Some(30);
        }
        for seed in ["hidden-nerfs-k1", "hidden-nerfs-k2", "hidden-nerfs-k3"] {
            let from_base = decide(&base, AI, &mut ai_options(seed, AI_BUDGET));
            assert!(from_base.is_some(), "{seed}");
            assert_eq!(
                decide(&changed, AI, &mut ai_options(seed, AI_BUDGET)),
                from_base,
                "{seed}"
            );
        }
    }
}

mod r1845002_the_ai_answers_a_trade_offer {
    use super::*;

    #[test]
    fn r1845002_the_ai_answers_both_trade_prompts() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({ "seed": "hidden-nerfs-trade", "p1": p1_side(), "p2": p2_side() }));
        // The offer, made for p1 by its field card as a Cry would make it.
        let maker = s.unit(AI, 1).expect("p1's unit");
        let mut rng = create_rng(&s.state().seed, s.state().rng_cursor);
        let mut events: Vec<GameEvent> = Vec::new();
        {
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            {
                let mut ctx = make_context(
                    &mut sink,
                    Some(&maker),
                    HookOptions {
                        controller: Some(AI),
                        ..HookOptions::default()
                    },
                );
                apply_effects(&[effects::trade_offer()], &mut ctx);
            }
            settle(&mut sink, SettleOptions::default());
        }
        s.state_mut().rng_cursor = rng.cursor();
        s.end_turn();

        for (seat, seed) in [(AI, "hidden-nerfs-give"), (HUMAN, "hidden-nerfs-take")] {
            let pending = s.state().pending.clone().expect("a trade prompt is open");
            assert_eq!(pending.player_id, seat);
            let action = decide(s.state(), seat, &mut ai_options(seed, AI_BUDGET))
                .map(|decision| decision.action)
                .expect("the AI answers");
            assert!(
                matches!(action, ActionBody::Answer { .. }),
                "{seat:?}: {action:?}"
            );
            assert!(is_legal(s.state(), seat, &action), "{seat:?}");
            s.act(action, seat, "the AI's answer");
        }
        assert!(s.state().pending.is_none());
        let moves = s
            .events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Stolen { .. }))
            .count();
        assert_eq!(moves, 2, "a card each way");
    }
}
