//! R5246000 (issue #5246): the AI counts each open Nerf on an enemy hand card and each misfire waiting on a
//! side's next Spell.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, clone, close, register_cards, scenario};

fn board(opts: Value) -> GameState {
    register_cards();
    let mut options = json!({ "seed": "r5246000-nerf-misfire", "active": AI, "turn": 1 });
    if let (Some(base), Value::Object(extra)) = (options.as_object_mut(), opts) {
        for (key, value) in extra {
            base.insert(key, value);
        }
    }
    scenario(options).state().clone()
}

fn eval(state: &GameState, seat: PlayerId) -> f64 {
    evaluate(state, seat, NextSwing::Enemy, &AI_EVAL)
}

/// The enemy's hand: 0 Mr. Vanilla ((1) 4/4), 1 Tempo Timmy (Rush, First Strike), 2 Stockpile (`draw` and
/// `heal`, better up), 3 Speciesist (`mana`, better down).
fn plain() -> GameState {
    board(json!({
        "p1": { "hand": ["core-005"], "library": ["core-008", "core-011"] },
        "p2": { "hand": ["core-008", "core-011", "core-005", "core-002"], "library": ["core-008", "core-011"] },
    }))
}

fn nerfed(state: &GameState, f: impl Fn(&mut Vec<CardInstance>)) -> GameState {
    let mut out = clone(state);
    f(&mut out.players[HUMAN].hand);
    out
}

fn misfires(state: &GameState, player: PlayerId, n: usize) -> GameState {
    let mut out = clone(state);
    for i in 0..n {
        out.players[player].mods.push(PlayerModifier {
            id: format!("mod-r5246000-{i}"),
            expiry: ModifierExpiry::Used,
            kind: ModifierKind::RandomTargetsNextSpell,
        });
    }
    out
}

fn stats(attack: i32, health: i32) -> Option<Tuning> {
    Some(Tuning {
        attack: Some(attack),
        health: Some(health),
        ..Default::default()
    })
}

fn steps(pairs: &[(&str, i32)]) -> Option<IndexMap<String, i32>> {
    Some(pairs.iter().map(|(key, step)| (key.to_string(), *step)).collect())
}

/// The enemy-side delta `k` Nerfs make, seen from the AI's seat.
fn counts(plain: &GameState, changed: &GameState, k: i32) {
    close(
        eval(changed, AI) - eval(plain, AI),
        AI_EVAL.enemy_hand_nerf * f64::from(k),
    );
}

#[test]
fn r5246000_a_cost_nerf_counts_a_step_a_crystal() {
    let plain = plain();
    for n in 1..=3 {
        let changed = nerfed(&plain, |hand| hand[0].cost_mod = n);
        counts(&plain, &changed, n);
        // The holder's own hand already prices the dearer card the same (`hand_cost_delta`, R65).
        close(
            eval(&plain, HUMAN) - eval(&changed, HUMAN),
            AI_EVAL.enemy_hand_nerf * f64::from(n),
        );
    }
}

#[test]
fn r5246000_a_stats_nerf_counts_a_step_a_stat_total() {
    let plain = plain();
    for ((attack, health), k) in [
        ((-2, -2), 1),
        ((-1, 0), 1),
        ((0, -3), 1),
        ((-4, -4), 2),
        ((-1, -4), 2),
    ] {
        let changed = nerfed(&plain, |hand| hand[1].tuning = stats(attack, health));
        counts(&plain, &changed, k);
    }
}

#[test]
fn r5246000_a_lost_or_harmful_keyword_counts_a_step() {
    let plain = plain();
    let lost = |kinds: Vec<KeywordKind>| {
        nerfed(&plain, move |hand| {
            hand[1].tuning = Some(Tuning {
                remove_keywords: Some(kinds.clone()),
                ..Default::default()
            })
        })
    };
    counts(&plain, &lost(vec![KeywordKind::Rush]), 1);
    counts(
        &plain,
        &lost(vec![KeywordKind::Rush, KeywordKind::FirstStrike]),
        2,
    );
    let crater = nerfed(&plain, |hand| {
        hand[1].tuning = Some(Tuning {
            add_keywords: Some(vec![Keyword::of_kind(KeywordKind::Crater, 0)]),
            ..Default::default()
        })
    });
    counts(&plain, &crater, 1);
}

#[test]
fn r5246000_a_number_moved_the_worse_way_counts_a_step() {
    let plain = plain();
    let numbers = |at: usize, pairs: &'static [(&'static str, i32)]| {
        nerfed(&plain, move |hand| {
            hand[at].tuning = Some(Tuning {
                numbers: steps(pairs),
                ..Default::default()
            })
        })
    };
    // Stockpile's numbers are better up, so a step down is a Nerf.
    counts(&plain, &numbers(2, &[("draw", -1)]), 1);
    counts(&plain, &numbers(2, &[("draw", -1), ("heal", -2)]), 3);
    // Speciesist's Overload is better down, so a step up is one.
    counts(&plain, &numbers(3, &[("mana", 1)]), 1);
    // More is better for every X but Tribute.
    let x = |pairs: &'static [(&'static str, i32)]| {
        nerfed(&plain, move |hand| {
            hand[0].tuning = Some(Tuning {
                x: steps(pairs),
                ..Default::default()
            })
        })
    };
    counts(&plain, &x(&[("Tribute", 1)]), 1);
    counts(&plain, &x(&[("Armor", -1)]), 1);
    counts(&plain, &x(&[("X", -2)]), 2);
}

#[test]
fn r5246000_nerfs_add_up_across_the_enemy_hand() {
    let plain = plain();
    let dearer = nerfed(&plain, |hand| {
        for card in hand.iter_mut() {
            card.cost_mod = 1;
        }
    });
    counts(&plain, &dearer, 4);
    let more = nerfed(&dearer, |hand| hand[1].tuning = stats(-2, -2));
    counts(&plain, &more, 5);
}

#[test]
fn r5246000_a_secret_nerf_or_an_upgrade_changes_nothing() {
    let plain = plain();
    let secret = nerfed(&plain, |hand| {
        for card in hand.iter_mut() {
            card.hidden_nerfs = Some(3);
        }
    });
    assert_eq!(eval(&secret, AI), eval(&plain, AI));

    let upgraded = nerfed(&plain, |hand| {
        hand[0].cost_mod = -1;
        hand[0].tuning = Some(Tuning {
            x: steps(&[("Tribute", -1), ("Armor", 1)]),
            ..Default::default()
        });
        hand[1].tuning = Some(Tuning {
            attack: Some(2),
            health: Some(2),
            remove_keywords: Some(vec![KeywordKind::Crater]),
            add_keywords: Some(vec![Keyword::Taunt]),
            ..Default::default()
        });
        hand[2].tuning = Some(Tuning {
            numbers: steps(&[("draw", 1)]),
            ..Default::default()
        });
        hand[3].tuning = Some(Tuning {
            numbers: steps(&[("mana", -1)]),
            ..Default::default()
        });
    });
    assert_eq!(eval(&upgraded, AI), eval(&plain, AI));

    // An Upgrade and a Nerf of the stats that cancel leave nothing below the face.
    let even = nerfed(&plain, |hand| hand[1].tuning = stats(3, -3));
    assert_eq!(eval(&even, AI), eval(&plain, AI));
}

#[test]
fn r5246000_a_live_misfire_lowers_its_side() {
    let plain = plain();
    for n in 1..=2 {
        let marked = misfires(&plain, HUMAN, n);
        let delta = AI_EVAL.random_targets * n as f64;
        close(eval(&marked, AI) - eval(&plain, AI), delta);
        close(eval(&plain, HUMAN) - eval(&marked, HUMAN), delta);
        // Outside the coming refreshes, so `replied_score` gives no credit back for one a reply spends.
        assert_eq!(
            coming_mana(&marked, AI, &AI_EVAL),
            coming_mana(&plain, AI, &AI_EVAL)
        );

        let own = misfires(&plain, AI, n);
        close(eval(&plain, AI) - eval(&own, AI), delta);
    }
}

#[test]
fn r5246000_a_spent_misfire_counts_nothing() {
    let plain = board(json!({
        "active": HUMAN,
        "p1": { "hand": ["core-008"], "library": ["core-008", "core-011"] },
        "p2": { "hand": ["core-005"], "mana": 1, "library": ["core-008", "core-011", "core-008"] },
    }));
    let marked = misfires(&plain, HUMAN, 1);
    let stockpile = plain.players[HUMAN].hand[0].id.clone();
    let play = legal_actions(&marked, HUMAN)
        .into_iter()
        .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == stockpile))
        .expect("Stockpile is a legal play");
    let spent = act(&marked, HUMAN, &play);
    let played = act(&plain, HUMAN, &play);
    assert!(
        !spent.players[HUMAN]
            .mods
            .iter()
            .any(|modifier| matches!(modifier.kind, ModifierKind::RandomTargetsNextSpell)),
        "the Spell spends the misfire"
    );
    close(eval(&spent, AI), eval(&played, AI));
}

#[test]
fn r5246000_greedy_eval_ignores_nerfs_and_misfires() {
    let plain = plain();
    let changed = misfires(&nerfed(&plain, |hand| hand[0].cost_mod = 2), HUMAN, 1);
    assert_eq!(
        evaluate(&changed, AI, NextSwing::Enemy, &GREEDY_EVAL),
        evaluate(&plain, AI, NextSwing::Enemy, &GREEDY_EVAL)
    );
}
