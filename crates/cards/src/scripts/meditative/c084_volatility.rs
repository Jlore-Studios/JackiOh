//! M #84 Volatility (SPEC §8.8 row 84): (1) Unit, Common, 1/1 → 2/2.
//!
//! Base:    "Cry: Buff this.\nBuffs and Nerfs are twice as effective on this."
//! Radiant: "Cry: Buff this.\nBuffs are three times as effective on this."
//! Engine: NEW: ME-TUNEMULT (R1160) — a static flag `tuneMultiplier`, read off the card's running
//! face wherever it is (field, hand, deck), by which each Buff or Nerf application on this card is
//! scaled: still one application and one draw, with the change N times as large. "Twice" and
//! "three times" are kept out of `params`, so a Buff can never raise its own multiplier. Patch v0.3.10
//! (#831) made it 1/1 and gave both faces "Cry: Buff this": one Upgrade of itself, which its own
//! multiplier then doubles (triples on the Radiant face).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-084";

/// Patch v0.3.10 (#831): "Cry: Buff this", one Buff on itself, which its own multiplier scales.
fn buff_this() -> Hook {
    hook(|_ctx| vec![upgrade(json_as(json!({ "target": { "of": "self" } })))])
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            static_flags: Some(StaticFlags {
                tune_multiplier: Some(TuneMultiplier {
                    upgrade: Some(VOLATILITY_TUNE_MULTIPLIER),
                    degrade: Some(VOLATILITY_TUNE_MULTIPLIER),
                }),
                ..StaticFlags::default()
            }),
            cry: Some(buff_this()),
            ..Script::default()
        },
        radiant: Script {
            static_flags: Some(StaticFlags {
                tune_multiplier: Some(TuneMultiplier {
                    upgrade: Some(VOLATILITY_RADIANT_BUFF_MULTIPLIER),
                    degrade: None,
                }),
                ..StaticFlags::default()
            }),
            cry: Some(buff_this()),
            ..Script::default()
        },
    }
}

// M #84 Volatility — SPEC §8.8 row 84, BUILD M10 row M 84: "One Buff on it is one draw whose change
// is doubled (MD-E7): a cost row of two steps (never below (0)), a stat split of twice the total,
// two keywords, X +2; a Nerf likewise doubled, never removing a harmful keyword; still one
// `upgraded` or `degraded` event; in hand and deck too; a +2/+2 stat buff and KY's Constant are not
// doubled; radiant 6/6, Buffs tripled and Nerfs normal". Patch v0.3.10: 1/1 → 2/2, and its Cry Buffs it.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const VOLATILITY: &str = "meditative-084";
    const OUTFITTER: &str = "meditative-082";
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    fn upgrades(events: &[GameEvent]) -> Vec<GameEvent> {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Upgraded { .. }))
            .cloned()
            .collect()
    }

    use jackioh_engine::effects::{TuneDirection, buff, tune_multiplier_of, tune_once};
    use jackioh_engine::resolve::{HookOptions, apply_effects, make_context};

    const P1: PlayerId = PlayerId::P1;
    const KY_CONSTANT: &str = "classicplus-041";
    const ARMORED: &str = "core-025"; // (4) 7/7 Armor 7.

    /// One Buff or Nerf of `id`, as one application of a tune verb makes it (B3.4 rule 1).
    fn tune(s: &mut Scenario, id: &str, direction: TuneDirection) -> Vec<GameEvent> {
        let card = s.card(id).clone();
        let state = s.state_mut();
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut *state, &mut events, &mut rng);
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(P1),
                    ..HookOptions::default()
                },
            );
            tune_once(&mut ctx, &card, direction, true);
        }
        state.rng_cursor = rng.cursor();
        events
    }

    /// `effects` run once with p1 as controller.
    fn apply(s: &mut Scenario, effects: Vec<Effect>) {
        let state = s.state_mut();
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut *state, &mut events, &mut rng);
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(P1),
                    ..HookOptions::default()
                },
            );
            apply_effects(&effects, &mut ctx);
        }
        state.rng_cursor = rng.cursor();
    }

    /// A Volatility in p1's hand (or `zone`), with `keywords` granted so the keyword and X rows
    /// have something to move.
    fn volatile(seed: &str, zone: &str, radiant: bool, keywords: &[Keyword]) -> (Scenario, String) {
        crate::register_all();
        let mut p1 = json!({ "hand": [FILLER], "library": filler(3) });
        let entry = json!({ "def": VOLATILITY, "radiant": radiant });
        match zone {
            "library" => p1["library"] = json!([entry, FILLER, FILLER]),
            "field" => p1["field"] = json!([entry]),
            _ => p1["hand"] = json!([entry, FILLER]),
        }
        let mut s = scenario(json!({
            "seed": seed,
            "p1": p1,
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        let id = s.state().players[P1]
            .hand
            .iter()
            .chain(s.state().players[P1].library.iter())
            .chain(s.state().players[P1].units.iter().flatten().flatten())
            .find(|card| card.def_id == VOLATILITY)
            .map(|card| card.id.clone())
            .expect("the Volatility");
        s.card_mut(id.as_str())
            .granted_keywords
            .extend(keywords.iter().cloned());
        (s, id)
    }

    fn harmless_and_harmful() -> Vec<Keyword> {
        vec![Keyword::Taunt, Keyword::Rush, Keyword::CantAttack]
    }

    /// A field Volatility with the 7/7's printed Armor 7 fused in (R102), so its X row (a
    /// printed numbered keyword) has something to move; its multipliers are its own.
    fn armored(seed: &str, radiant: bool, keywords: &[Keyword]) -> (Scenario, String) {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [FILLER],
                "library": filler(3),
                "field": [{ "def": VOLATILITY, "radiant": radiant }, ARMORED],
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        let kept = s.unit(P1, 1).unwrap();
        let other = s.unit(P1, 2).unwrap();
        {
            let state = s.state_mut();
            let mut events = Vec::new();
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut sink = EngineSink::new(&mut *state, &mut events, &mut rng);
            jackioh_engine::subsystems::fuse::fuse(
                &mut sink,
                json_as(json!({ "ingredients": [other], "target": kept })),
            )
            .expect("the fusion");
        }
        s.card_mut(kept.id.as_str())
            .granted_keywords
            .extend(keywords.iter().cloned());
        (s, kept.id)
    }

    /// The one tune event of a step, panicking unless there is exactly one.
    fn only(events: &[GameEvent]) -> TuningChange {
        let changes: Vec<TuningChange> = events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Upgraded { change, .. } | GameEvent::Degraded { change, .. } => {
                    Some(change.clone())
                }
                _ => None,
            })
            .collect();
        assert_eq!(changes.len(), 1, "one application is one event: {changes:?}");
        changes[0].clone()
    }

    mod m84_volatility {
        use super::*;

        #[test]
        fn is_a_1_1_1_common_unit_radiant_2_2_with_no_params() {
            let def = crate::card_def(ID);
            assert_eq!(def.id, VOLATILITY);
            assert_eq!(def.cost, CardCost::Fixed(1));
            assert_eq!(def.type_, CardType::Unit);
            assert_eq!(def.rarity, Rarity::Common);
            assert!(def.tags.is_empty());
            assert_eq!(
                [
                    def.base.attack,
                    def.base.health,
                    def.radiant.attack,
                    def.radiant.health
                ],
                [Some(1), Some(1), Some(2), Some(2)]
            );
            assert!(def.params.is_none() || def.params.as_ref().is_some_and(Vec::is_empty));
        }

        #[test]
        fn r1160_both_faces_carry_their_multipliers() {
            let scripts = script();
            assert_eq!(
                scripts.base.static_flags.and_then(|flags| flags.tune_multiplier),
                Some(TuneMultiplier {
                    upgrade: Some(VOLATILITY_TUNE_MULTIPLIER),
                    degrade: Some(VOLATILITY_TUNE_MULTIPLIER),
                })
            );
            assert_eq!(
                scripts
                    .radiant
                    .static_flags
                    .and_then(|flags| flags.tune_multiplier),
                Some(TuneMultiplier {
                    upgrade: Some(VOLATILITY_RADIANT_BUFF_MULTIPLIER),
                    degrade: None,
                })
            );
        }

        #[test]
        fn r1160_a_buff_from_medina_outfitter_in_hand_is_one_event_doubled() {
            // One Buff from an Outfitter's Cry, across the seeds: still one event, with a doubled
            // change — stats of 8, two keywords, or cost at its floor.
            let mut stats = 0;
            let mut keyword = 0;
            let mut cost = 0;
            for seed in 1..=40 {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": format!("vol-cry-{seed}"),
                    "p1": {
                        "mana": 10,
                        "hand": [OUTFITTER, { "def": VOLATILITY }],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(OUTFITTER, json!({}));
                let ups = upgrades(s.events());
                assert_eq!(ups.len(), 1, "one application is one event (seed {seed})");
                match &ups[0] {
                    GameEvent::Upgraded {
                        change: TuningChange::Stats { attack, health },
                        ..
                    } => {
                        assert_eq!(attack + health, 2 * TUNE_STAT_TOTAL);
                        stats += 1;
                    }
                    GameEvent::Upgraded {
                        change: TuningChange::Keyword { also, .. },
                        ..
                    } => {
                        assert_eq!(also.as_ref().map(Vec::len), Some(1));
                        keyword += 1;
                    }
                    GameEvent::Upgraded {
                        change: TuningChange::Cost { delta },
                        ..
                    } => {
                        // Cost (1), doubled to −2, floored at (0): −1 is what moved.
                        assert_eq!(*delta, -1);
                        cost += 1;
                    }
                    other => panic!("unexpected event {other:?} (seed {seed})"),
                }
            }
            assert!(stats > 0 && keyword > 0 && cost > 0, "each doubled row is drawn");
        }

        #[test]
        fn r1160_a_buffs_cost_row_moves_two_steps_never_below_0() {
            let (mut two, mut floored) = (false, false);
            for seed in 1..=40 {
                // (3): two steps reach (1).
                let (mut s, id) = volatile(&format!("vol-cost-{seed}"), "hand", false, &[]);
                s.card_mut(id.as_str()).cost_mod = 2;
                if let TuningChange::Cost { delta } = only(&tune(&mut s, &id, TuneDirection::Upgrade)) {
                    assert_eq!(delta, -2);
                    assert_eq!(s.card(id.as_str()).cost_mod, 0);
                    two = true;
                }
                // (1): the second step would pass (0), so only one moves.
                let (mut s, id) = volatile(&format!("vol-floor-{seed}"), "hand", false, &[]);
                if let TuningChange::Cost { delta } = only(&tune(&mut s, &id, TuneDirection::Upgrade)) {
                    assert_eq!(delta, -1);
                    assert_eq!(cost_now(s.state(), s.card(id.as_str())), 0);
                    floored = true;
                }
            }
            assert!(two && floored, "the cost row was drawn both ways");
        }

        #[test]
        fn r1160_a_buffs_stat_row_splits_twice_the_total() {
            let mut seen = false;
            for seed in 1..=40 {
                let (mut s, id) = volatile(&format!("vol-stats-{seed}"), "field", false, &[]);
                if let TuningChange::Stats { attack, health } =
                    only(&tune(&mut s, &id, TuneDirection::Upgrade))
                {
                    assert_eq!(attack + health, 2 * TUNE_STAT_TOTAL);
                    let view = s.stats(id.as_str());
                    assert_eq!((view.attack, view.health), (1 + attack, 1 + health));
                    seen = true;
                }
            }
            assert!(seen, "the stat row was drawn");
        }

        #[test]
        fn r1160_a_buffs_keyword_row_adds_two_keywords() {
            let mut seen = false;
            for seed in 1..=40 {
                let (mut s, id) = volatile(&format!("vol-kw-{seed}"), "field", false, &[]);
                if let TuningChange::Keyword { keyword, added, also } =
                    only(&tune(&mut s, &id, TuneDirection::Upgrade))
                {
                    assert!(added);
                    let also = also.expect("a doubled add moves two keywords");
                    assert_eq!(also.len(), 1);
                    assert_ne!(also[0].kind(), keyword.kind());
                    let kinds: Vec<KeywordKind> =
                        s.stats(id.as_str()).keywords.iter().map(Keyword::kind).collect();
                    assert!(kinds.contains(&keyword.kind()) && kinds.contains(&also[0].kind()));
                    seen = true;
                }
            }
            assert!(seen, "the keyword row was drawn");
        }

        #[test]
        fn r1160_a_buffs_x_row_moves_x_by_2() {
            let mut seen = false;
            for seed in 1..=60 {
                let (mut s, id) = armored(&format!("vol-x-{seed}"), false, &[]);
                let armor = s.stats(id.as_str()).armor;
                if let TuningChange::X { key, delta } = only(&tune(&mut s, &id, TuneDirection::Upgrade)) {
                    assert_eq!(key, "Armor");
                    assert_eq!(delta, 2);
                    assert_eq!(s.stats(id.as_str()).armor, armor + 2);
                    seen = true;
                }
            }
            assert!(seen, "the X row was drawn");
        }

        #[test]
        fn r1160_a_nerf_is_doubled_and_never_removes_a_harmful_keyword() {
            let (mut keyword_seen, mut x_seen) = (false, false);
            for seed in 1..=60 {
                let (mut s, id) = armored(&format!("vol-nerf-{seed}"), false, &harmless_and_harmful());
                let armor = s.stats(id.as_str()).armor;
                let events = tune(&mut s, &id, TuneDirection::Degrade);
                assert_eq!(
                    events
                        .iter()
                        .filter(|event| matches!(event, GameEvent::Degraded { .. }))
                        .count(),
                    1,
                    "still one `degraded` event"
                );
                match only(&events) {
                    TuningChange::Keyword { keyword, added, also } => {
                        assert!(!added);
                        let also = also.expect("a doubled remove takes two keywords");
                        assert_eq!(also.len(), 1);
                        for moved in std::iter::once(&keyword).chain(also.iter()) {
                            assert_ne!(moved.kind(), KeywordKind::CantAttack, "never a harmful one");
                        }
                        assert!(
                            s.stats(id.as_str())
                                .keywords
                                .iter()
                                .any(|k| k.kind() == KeywordKind::CantAttack),
                            "Can't attack stays"
                        );
                        keyword_seen = true;
                    }
                    TuningChange::X { key, delta } => {
                        assert_eq!(key, "Armor");
                        assert_eq!(delta, -2);
                        assert_eq!(s.stats(id.as_str()).armor, armor - 2);
                        x_seen = true;
                    }
                    _ => {}
                }
            }
            assert!(keyword_seen && x_seen, "the keyword and X rows were drawn");
        }

        #[test]
        fn r1160_doubled_in_hand_deck_and_field() {
            for zone in ["hand", "library", "field"] {
                let (s, id) = volatile(&format!("vol-zone-{zone}"), zone, false, &[]);
                let card = s.card(id.as_str()).clone();
                assert_eq!(
                    tune_multiplier_of(s.state(), &card, TuneDirection::Upgrade),
                    2,
                    "{zone}"
                );
                assert_eq!(
                    tune_multiplier_of(s.state(), &card, TuneDirection::Degrade),
                    2,
                    "{zone}"
                );
            }
            // A Buff in the deck is doubled as it is in hand.
            let mut seen = false;
            for seed in 1..=40 {
                let (mut s, id) = volatile(&format!("vol-deck-{seed}"), "library", false, &[]);
                if let TuningChange::Stats { attack, health } =
                    only(&tune(&mut s, &id, TuneDirection::Upgrade))
                {
                    assert_eq!(attack + health, 2 * TUNE_STAT_TOTAL);
                    seen = true;
                }
            }
            assert!(seen, "a deck Buff's stat row was drawn");
        }

        #[test]
        fn r1160_a_plus_2_plus_2_stat_buff_is_not_doubled() {
            let (mut s, id) = volatile("vol-plain-buff", "field", false, &[]);
            apply(
                &mut s,
                vec![buff(json_as(json!({
                    "target": { "of": "instance", "instanceId": id },
                    "attack": 2,
                    "health": 2,
                })))],
            );
            s.expect_stats(id.as_str(), json!({ "attack": 3, "health": 3 }));
        }

        #[test]
        fn r1160_kys_constant_is_not_doubled() {
            // KY's Constant sets a random number on a hand card to 3: Volatility's numbers are its
            // cost (1) and its 1/1 (patch v0.3.10), and the one picked goes to 3 outright, not twice as far.
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "vol-constant",
                "p1": { "hand": [KY_CONSTANT, VOLATILITY, FILLER], "library": filler(3) },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            let id = s.card(VOLATILITY).id.clone();
            s.play(
                KY_CONSTANT,
                json!({ "targets": [{ "pick": "instance", "instanceId": id }] }),
            );
            let card = s.card(id.as_str()).clone();
            let Some(stats) = jackioh_engine::current_stats(s.state(), &card) else {
                panic!("Volatility has stats");
            };
            let mut numbers = vec![cost_now(s.state(), &card), stats.attack, stats.health];
            numbers.sort();
            assert_eq!(numbers, vec![1, 1, 3]);
        }

        #[test]
        fn r1160_radiant_nerfs_are_normal() {
            let (mut keyword_seen, mut x_seen) = (false, false);
            for seed in 1..=60 {
                let (mut s, id) = armored(&format!("vol-rnerf-{seed}"), true, &harmless_and_harmful());
                let card = s.card(id.as_str()).clone();
                assert_eq!(tune_multiplier_of(s.state(), &card, TuneDirection::Degrade), 1);
                match only(&tune(&mut s, &id, TuneDirection::Degrade)) {
                    TuningChange::Keyword { added, also, .. } => {
                        assert!(!added);
                        assert_eq!(also, None, "a plain Nerf moves one keyword");
                        keyword_seen = true;
                    }
                    TuningChange::X { delta, .. } => {
                        assert_eq!(delta, -1);
                        x_seen = true;
                    }
                    _ => {}
                }
            }
            assert!(keyword_seen && x_seen);
        }

        #[test]
        fn radiant_is_2_2() {
            let (mut s, id) = volatile("vol-radiant-stats", "field", true, &[]);
            s.expect_stats(id.as_str(), json!({ "attack": 2, "health": 2 }));
        }

        #[test]
        fn r1160_radiant_a_buff_is_tripled() {
            let mut tripled = 0;
            for seed in 1..=20 {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": format!("vol-radiant-{seed}"),
                    "p1": {
                        "mana": 10,
                        "hand": [OUTFITTER, { "def": VOLATILITY, "radiant": true }],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(OUTFITTER, json!({}));
                let ups = upgrades(s.events());
                assert_eq!(ups.len(), 1);
                match &ups[0] {
                    GameEvent::Upgraded {
                        change: TuningChange::Stats { attack, health },
                        ..
                    } => {
                        assert_eq!(attack + health, 3 * TUNE_STAT_TOTAL);
                        tripled += 1;
                    }
                    GameEvent::Upgraded {
                        change: TuningChange::Keyword { also, .. },
                        ..
                    } => {
                        assert_eq!(also.as_ref().map(Vec::len), Some(2));
                        tripled += 1;
                    }
                    // The cost row (1, tripled to −3, floored) and the X row (no X here) are
                    // covered by the engine's own R1160 tests; any tripled row counts.
                    GameEvent::Upgraded { .. } => {
                        tripled += 1;
                    }
                    other => panic!("unexpected event {other:?}"),
                }
            }
            assert!(tripled > 0);
        }

        /// The one `upgraded` event the Cry of a Volatility played from hand makes, over `seeds`, and
        /// each stat split it drew.
        fn cried(radiant: bool) -> (usize, Vec<i32>) {
            let mut stat_totals = Vec::new();
            let mut cries = 0;
            for seed in 1..=20 {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": format!("vol-cry-{radiant}-{seed}"),
                    "p1": { "hand": [{ "def": VOLATILITY, "radiant": radiant }, FILLER], "library": filler(3) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                let id = s.card(VOLATILITY).id.clone();
                s.play(VOLATILITY, json!({}));
                let ups = upgrades(s.events());
                assert_eq!(ups.len(), 1, "one Buff, on itself");
                let GameEvent::Upgraded { instance_id, change, .. } = &ups[0] else {
                    unreachable!("upgrades() keeps upgraded events")
                };
                assert_eq!(instance_id, &id);
                cries += 1;
                if let TuningChange::Stats { attack, health } = change {
                    stat_totals.push(attack + health);
                    s.expect_stats(id.as_str(), json!({ "attack": (if radiant { 2 } else { 1 }) + attack, "health": (if radiant { 2 } else { 1 }) + health }));
                }
            }
            (cries, stat_totals)
        }

        #[test]
        fn its_cry_buffs_it_once_and_the_buff_is_doubled() {
            let (cries, totals) = cried(false);
            assert_eq!(cries, 20);
            assert!(!totals.is_empty(), "the stat row was drawn");
            assert!(totals.iter().all(|total| *total == 2 * TUNE_STAT_TOTAL), "{totals:?}");
        }

        #[test]
        fn radiant_its_cry_buffs_it_once_and_the_buff_is_tripled() {
            let (cries, totals) = cried(true);
            assert_eq!(cries, 20);
            assert!(!totals.is_empty(), "the stat row was drawn");
            assert!(totals.iter().all(|total| *total == 3 * TUNE_STAT_TOTAL), "{totals:?}");
        }
    }
}
