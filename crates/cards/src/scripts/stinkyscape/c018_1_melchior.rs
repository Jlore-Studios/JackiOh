//! S #18.1 MELCHIOR (SPEC §8.10 row S 18.1, §7; §6.1, §6.3, R21, R62, R384, R781005, R1839000,
//! R1843000, R1866001): (1) Field Spell, Tech, Token (printed Legendary), summoned by S #18 Imitated
//! Instrumentality.
//!   Base:    "Indestructible / Start of turn: Vote to remove you if your opponent has more Units than
//!            you. / Activate: Discover a combat keyword. Give it to each of your Units."
//!   Radiant: "Indestructible / Start of turn: Vote to remove you if your opponent has more Units than
//!            you. / Activate: Give each of your Units {keywords} random combat keywords." (3)
//!
//! Engine: Indestructible is printed in `catalog.json` (§6.1). Start of turn (R62): `vote_out` when its
//! controller's opponent has strictly more Units than they do, as `active_units_of` counts them, so a
//! Dormant card is no Unit (R1839000) (R1843000). Activate (R384, once a turn): a Discover of three
//! different combat keywords drawn from R21's pool (R781005), offered as a `mode` prompt, the pick
//! granted to each of its controller's Units as the answer resolves (`grant_keyword`); with no Unit,
//! nothing is asked.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-018-1";

/// The step the Discover's answer re-enters (§10.6).
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

/// The base Activate: three different combat keywords on offer (§6.3 Discover).
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

/// The pick, given to each Unit its controller has as the answer resolves.
fn give_picked(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(picked) = chosen_options(ctx).into_iter().next() else {
        return vec![];
    };
    your_units(ctx)
        .iter()
        .map(|id| grant_keyword(json_as(json!({ "target": instance(id), "keyword": { "kind": picked } }))))
        .collect()
}

/// The Radiant Activate: each Unit draws its own `keywords` (R21).
fn give_random(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let count = param(&*ctx, "keywords");
    your_units(ctx)
        .iter()
        .map(|id| grant_random_keywords(json_as(json!({ "target": instance(id), "count": count }))))
        .collect()
}

fn melchior(radiant: bool) -> Script {
    let activate = ActivationDecl {
        // One id on both faces, so a face changed mid-turn keeps the turn's use (R384).
        id: "keywords".to_string(),
        label: if radiant {
            "Give each of your Units random combat keywords".to_string()
        } else {
            "Discover a combat keyword. Give it to each of your Units".to_string()
        },
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: if radiant { hook(give_random) } else { hook(discover) },
    };
    let mut script = Script {
        start_of_turn: Some(hook(vote_if_outnumbered)),
        activations: vec![activate],
        ..Script::default()
    };
    if !radiant {
        script.resume.insert(PICKED, hook(give_picked));
    }
    script
}

pub fn script() -> CardScripts {
    CardScripts {
        base: melchior(false),
        radiant: melchior(true),
    }
}

// S #18.1 MELCHIOR — SPEC §8.10 row S 18.1, BUILD M12 row S 18.1 (R1843000, R1866001).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

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

        #[test]
        fn r1866001_activate_discovers_three_different_combat_keywords_and_gives_the_pick_to_each_of_your_units() {
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
        }

        #[test]
        fn r1866001_with_no_unit_it_asks_nothing() {
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
            let options = offered(&s);
            s.answer(json!(options[1]));
            s.expect_refused(|s| s.activate(melchior.as_str(), json!({ "ability": "keywords" })));
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

        fn gained(seed: &str) -> Vec<Vec<String>> {
            let mut s = scenario(json!({
                "seed": seed,
                "p1": {
                    "hand": [FILLER],
                    "backrow": [{ "def": MELCHIOR, "radiant": true }],
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

        #[test]
        fn r1866001_radiant_each_unit_gains_its_own_three_random_combat_keywords() {
            let mut differ = false;
            for seed in 0..8 {
                let sets = gained(&format!("melchior-radiant-{seed}"));
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
