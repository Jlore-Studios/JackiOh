//! S #18.1 MELCHIOR (SPEC §8.10 row S 18.1, §7; §6.1, §6.3, R21, R62, R384, R781005, R1839000,
//! R1843000, R1866001, R3142001): (1) Field Spell, Tech, Token (printed Legendary), summoned by S #18
//! Imitated Instrumentality.
//!   Base:    "Indestructible / Start of turn: Vote to remove you if your opponent has more Units than
//!            you. / Activate: Give each of your Units {keywords} random combat keyword." (1)
//!   Radiant: "Indestructible / Start of turn: Vote to remove you if your opponent has more Units than
//!            you. / Activate: Give each of your Units {keywords} random combat keywords." (3)
//!
//! Engine: Indestructible is printed in `catalog.json` (§6.1). Start of turn (R62): `vote_out` when its
//! controller's opponent has strictly more Units than they do, as `active_units_of` counts them, so a
//! Dormant card is no Unit (R1839000) (R1843000). Activate (R384, once a turn): each of its controller's
//! Units draws its own `keywords` random combat keywords from R21's pool (`grant_random_keywords`), 1 on
//! the base face and 3 on the Radiant one, asking nothing (R3142001). Through v0.5.11r the base face
//! Discovered one combat keyword, a `mode` prompt of three different ones drawn from the pool (R781005),
//! and gave the pick to each Unit as the answer resolved (`script_v0_5_11o`, kept, R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-018-1";

/// The step the Discover's answer re-enters (§10.6); only the script kept for v0.5.11r and before has it.
const PICKED: &str = "picked";

/// R1866001: "votes to remove you if your opponent has more Units than you".
fn vote_if_outnumbered(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let me = ctx.controller;
    if active_units_of(ctx.state, opponent_of(me)).len() > active_units_of(ctx.state, me).len() {
        vec![vote_out(Default::default())]
    } else {
        vec![]
    }
}

/// The ids of this player's Units, in R68's order.
fn your_units(ctx: &EffectContext<'_>) -> Vec<String> {
    active_units_of(ctx.state, ctx.controller).iter().map(|unit| unit.id.clone()).collect()
}

fn instance(id: &str) -> Value {
    json!({ "of": "instance", "instanceId": id })
}

/// The base Activate through v0.5.11r: three different combat keywords on offer (§6.3 Discover).
fn discover(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    if your_units(ctx).is_empty() {
        return vec![];
    }
    let offered: Vec<&str> =
        ctx.rng.shuffle(RANDOM_KEYWORD_POOL).into_iter().take(DISCOVER_OPTIONS as usize).collect();
    vec![choose_mode(json_as(json!({
        "options": offered,
        "step": PICKED,
        "prompt": "MELCHIOR: Discover a combat keyword for each of your Units",
    })))]
}

/// The pick of the base Activate through v0.5.11r, given to each Unit its controller has as the answer
/// resolves.
fn give_picked(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(picked) = chosen_options(ctx).into_iter().next() else {
        return vec![];
    };
    your_units(ctx)
        .iter()
        .map(|id| grant_keyword(json_as(json!({ "target": instance(id), "keyword": { "kind": picked } }))))
        .collect()
}

/// The Activate: each Unit draws its own `keywords` (R21, R3142001).
fn give_random(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let count = param(&*ctx, "keywords");
    your_units(ctx)
        .iter()
        .map(|id| grant_random_keywords(json_as(json!({ "target": instance(id), "count": count }))))
        .collect()
}

/// One face. `discovers` is the base face as it was through v0.5.11r (R3142001): it Discovers a keyword.
fn melchior(radiant: bool, discovers: bool) -> Script {
    let activate = ActivationDecl {
        // One id on both faces, so a face changed mid-turn keeps the turn's use (R384).
        id: "keywords".to_string(),
        label: if discovers {
            "Discover a combat keyword. Give it to each of your Units".to_string()
        } else if radiant {
            "Give each of your Units random combat keywords".to_string()
        } else {
            "Give each of your Units a random combat keyword".to_string()
        },
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: if discovers { hook(discover) } else { hook(give_random) },
    };
    let mut script = Script {
        start_of_turn: Some(hook(vote_if_outnumbered)),
        activations: vec![activate],
        ..Script::default()
    };
    if discovers {
        script.resume.insert(PICKED, hook(give_picked));
    }
    script
}

pub fn script() -> CardScripts {
    CardScripts {
        base: melchior(false, false),
        radiant: melchior(true, false),
    }
}

/// R767000: through v0.5.11r the base face Discovered a combat keyword and gave it to each Unit
/// (R3142001); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    CardScripts {
        base: melchior(false, true),
        radiant: melchior(true, false),
    }
}

// S #18.1 MELCHIOR — SPEC §8.10 row S 18.1, BUILD M12 row S 18.1 (R1843000, R1866001, R3142001).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::script_v0_5_11o;
    use crate::{filler, scenario, under_kept};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MELCHIOR: &str = "stinkyscape-018-1";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4, no keyword.
    const FILLER: &str = "core-016"; // Hit Job: inert in a hand.
    const HONEYPOT: &str = "core-060"; // A Trap to set beside it.
    const GUY_ATT: &str = "classicplus-005"; // Cry: Destroy every backrow card you control.

    fn table(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [FILLER],
                "backrow": [{ "def": MELCHIOR, "radiant": radiant }],
                "field": mine,
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "field": theirs, "library": filler(4) },
        }))
    }

    fn votes(s: &Scenario) -> usize {
        s.last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::VoteCast { def_id, .. } if def_id == MELCHIOR))
            .count()
    }

    fn votes_at_start(mine: Value, theirs: Value) -> usize {
        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "hand": [FILLER], "backrow": [MELCHIOR], "field": mine, "library": filler(4) },
            "p2": { "hand": [FILLER], "field": theirs, "library": filler(4) },
        }));
        s.end_turn();
        assert_eq!(s.state().active, P1);
        votes(&s)
    }

    fn melchior_id(s: &Scenario) -> String {
        s.card(MELCHIOR).id.clone()
    }

    fn keywords(s: &Scenario, id: &str) -> Vec<String> {
        s.stats(id).keywords.iter().map(keyword_key).collect()
    }

    fn units(s: &Scenario, player: PlayerId) -> Vec<String> {
        active_units_of(s.state(), player).iter().map(|unit| unit.id.clone()).collect()
    }

    // Differs from Scenario::offered: the open Mode prompt's options, not the seat's legal actions.
    fn offered(s: &Scenario) -> Vec<String> {
        let pending = s.state().pending.as_ref().expect("MELCHIOR's Discover");
        assert_eq!(pending.kind, PromptKind::Mode);
        assert_eq!(pending.player_id, P1);
        pending
            .options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Mode { option } => Some(option.clone()),
                _ => None,
            })
            .collect()
    }

    /// The keywords each of p1's two Units has after the Activate, the opponent's Unit checked bare and
    /// nothing asked.
    fn gained(radiant: bool, seed: &str) -> Vec<Vec<String>> {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [FILLER],
                "backrow": [{ "def": MELCHIOR, "radiant": radiant }],
                "field": [VANILLA, VANILLA],
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "field": [VANILLA], "library": filler(4) },
        }));
        s.activate(melchior_id(&s).as_str(), json!({ "ability": "keywords" }));
        assert!(s.state().pending.is_none(), "nothing is asked");
        for unit in units(&s, P2) {
            assert!(keywords(&s, &unit).is_empty(), "{unit}");
        }
        units(&s, P1).iter().map(|unit| keywords(&s, unit)).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r1866001_votes_only_while_your_opponent_has_more_units() {
            assert_eq!(votes_at_start(json!([VANILLA]), json!([VANILLA, VANILLA])), 1);
            assert_eq!(votes_at_start(json!([VANILLA, VANILLA]), json!([VANILLA, VANILLA])), 0);
            // R1839000: a Dormant card is no Unit.
            assert_eq!(
                votes_at_start(json!([VANILLA]), json!([VANILLA, { "def": VANILLA, "slumber": { "turns": 2 } }])),
                0
            );
        }

        /// R3142001: no Discover; each Unit draws its own one combat keyword from R21's pool, so the Units
        /// differ across seeds.
        #[test]
        fn r3142001_activate_gives_each_of_your_units_its_own_random_combat_keyword_and_asks_nothing() {
            let mut differ = false;
            for seed in 0..8 {
                let sets = gained(false, &format!("melchior-base-{seed}"));
                assert_eq!(sets.len(), 2);
                for set in &sets {
                    assert_eq!(set.len(), 1, "{set:?}");
                    assert!(set.iter().all(|key| RANDOM_KEYWORD_POOL.contains(&key.as_str())), "{set:?}");
                }
                differ |= sets[0] != sets[1];
            }
            assert!(differ, "each Unit draws its own");
        }

        #[test]
        fn r3142001_with_no_unit_it_gives_nothing_and_the_turns_use_is_spent() {
            let mut s = table(false, json!([]), json!([VANILLA]));
            let melchior = melchior_id(&s);
            s.activate(melchior.as_str(), json!({ "ability": "keywords" }));
            assert!(s.state().pending.is_none());
            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::KeywordGranted));
            // R384: the turn's use is spent all the same.
            s.expect_refused(|s| s.activate(melchior.as_str(), json!({ "ability": "keywords" })));
        }

        #[test]
        fn r1866001_activates_once_a_turn() {
            let mut s = table(false, json!([VANILLA]), json!([]));
            let melchior = melchior_id(&s);
            s.activate(melchior.as_str(), json!({ "ability": "keywords" }));
            assert!(s.state().pending.is_none(), "nothing is asked");
            s.expect_refused(|s| s.activate(melchior.as_str(), json!({ "ability": "keywords" })));
        }

        #[test]
        fn r3142001_keywords_reads_through_param_and_is_1_on_the_base_face() {
            let mut s = table(false, json!([VANILLA]), json!([]));
            assert_eq!(crate::upgrade_number(&mut s, MELCHIOR, "keywords"), 2);
            s.activate(melchior_id(&s).as_str(), json!({ "ability": "keywords" }));
            let unit = units(&s, P1)[0].clone();
            assert_eq!(keywords(&s, &unit).len(), 2);
        }

        /// R767000: through v0.5.11r the base Activate Discovered three different combat keywords from
        /// R21's pool and gave the pick to each of its controller's Units.
        #[test]
        fn r767000_the_kept_script_discovers_a_combat_keyword_and_gives_the_pick_to_each_of_your_units() {
            under_kept(MELCHIOR, script_v0_5_11o(), || {
                let mut s = table(false, json!([VANILLA, VANILLA]), json!([VANILLA]));
                s.activate(melchior_id(&s).as_str(), json!({ "ability": "keywords" }));
                let options = offered(&s);
                assert_eq!(options.len(), 3, "{options:?}");
                assert_eq!(options.iter().collect::<IndexSet<_>>().len(), 3, "{options:?}");
                assert!(options.iter().all(|option| RANDOM_KEYWORD_POOL.contains(&option.as_str())), "{options:?}");
                // §6.3 Discover: the choice is shown to its controller only.
                assert_eq!(crate::js(&s.view(P2))["pending"]["forYou"], json!(false));

                s.answer(json!(options[0]));
                for unit in units(&s, P1) {
                    assert_eq!(keywords(&s, &unit), [options[0].clone()], "{unit}");
                }
                for unit in units(&s, P2) {
                    assert!(keywords(&s, &unit).is_empty(), "{unit}");
                }

                // Radiant, kept as it is: three random ones each, no prompt.
                let mut s = table(true, json!([VANILLA]), json!([]));
                s.activate(melchior_id(&s).as_str(), json!({ "ability": "keywords" }));
                assert!(s.state().pending.is_none());
                assert_eq!(keywords(&s, &units(&s, P1)[0]).len(), 3);
            });
        }

        #[test]
        fn r1866001_indestructible_a_destroy_leaves_it() {
            let mut s = scenario(json!({
                "p1": { "hand": [GUY_ATT, FILLER], "backrow": [MELCHIOR, HONEYPOT], "library": filler(4) },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            s.play(GUY_ATT, json!({}));
            s.expect_in_zone(HONEYPOT, "graveyard");
            s.expect_in_zone(MELCHIOR, "field");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1866001_radiant_each_unit_gains_its_own_three_random_combat_keywords() {
            let mut differ = false;
            for seed in 0..8 {
                let sets = gained(true, &format!("melchior-radiant-{seed}"));
                assert_eq!(sets.len(), 2);
                for set in &sets {
                    assert_eq!(set.len(), 3, "{set:?}");
                    assert_eq!(set.iter().collect::<IndexSet<_>>().len(), 3, "{set:?}");
                    assert!(set.iter().all(|key| RANDOM_KEYWORD_POOL.contains(&key.as_str())), "{set:?}");
                }
                differ |= sets[0].iter().collect::<IndexSet<_>>() != sets[1].iter().collect::<IndexSet<_>>();
            }
            assert!(differ, "each Unit draws its own");
        }

        #[test]
        fn keywords_reads_through_param() {
            let mut s = table(true, json!([VANILLA]), json!([]));
            assert_eq!(crate::upgrade_number(&mut s, MELCHIOR, "keywords"), 4);
            s.activate(melchior_id(&s).as_str(), json!({ "ability": "keywords" }));
            let unit = units(&s, P1)[0].clone();
            assert_eq!(keywords(&s, &unit).len(), 4);
        }

        #[test]
        fn r1866001_radiant_activates_once_a_turn() {
            let mut s = table(true, json!([VANILLA]), json!([]));
            let melchior = melchior_id(&s);
            s.activate(melchior.as_str(), json!({ "ability": "keywords" }));
            s.expect_refused(|s| s.activate(melchior.as_str(), json!({ "ability": "keywords" })));
        }
    }
}
