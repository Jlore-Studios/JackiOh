//! S #4.6 The Big Crunch (SPEC §8.10 row S 4.6, SPEC §7; §6.3 Cast, R28, R70, R87, R423, R436, R452,
//! R1420, R1857002, R1870006): (4) Spell, Terminus, Token (printed Mythic), one of Olivian Expanse's
//! picks.
//!   Base:    "Cast one of each Call to Chaos."
//!   Radiant: "Cast a Radiant one of each Call to Chaos."
//!
//! Engine: one `cast_new` per Call to Chaos definition, in catalog order (`editions()`): Core #95, C+ #73,
//! M #95, the set's own two, then TG #39 once its set ships (R2200004). Each is a play for every
//! rule, its choices its caster's (R70), rolling and naming its own effects to both players (R436),
//! its nested casts held by `CALL_TO_CHAOS_CHAIN_CAP` (R28) and `RANDOM_CAST_CHAIN_CAP` (R452); each
//! lands in its caster's graveyard after it resolves (R87). The Radiant face casts each Radiant, three
//! effects each (R423) (R1870006).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-004-6";

/// Every Call to Chaos the catalog holds, in its order.
pub const EDITIONS: [&str; 5] = [
    "core-095",
    "classicplus-073",
    "meditative-095",
    "stinkyscape-150",
    "stinkyscape-151",
];

/// R2200004: the editions of sets still held (R1420), each cast after `EDITIONS` once the Call to Chaos
/// pool holds it, i.e. once its set ships: Training Grounds #39.
pub const HELD_EDITIONS: [&str; 1] = ["traininggrounds-039"];

/// `EDITIONS`, then each of `HELD_EDITIONS` the Call to Chaos pool holds now.
pub fn editions() -> Vec<&'static str> {
    let pool = jackioh_engine::catalog::query(&json_as(json!({ "tags": ["Call to Chaos"] })));
    let shipped = |def_id: &&str| pool.iter().any(|def| def.id == *def_id);
    EDITIONS
        .iter()
        .copied()
        .chain(HELD_EDITIONS.iter().copied().filter(shipped))
        .collect()
}

fn crunch(radiant: bool) -> Hook {
    hook(move |_ctx| {
        editions()
            .iter()
            .map(|def_id| {
                cast_new(CastNewArgs {
                    def: CastNewDef::Def(CastDef {
                        def_id: (*def_id).to_string(),
                        radiant: Some(radiant),
                    }),
                    radiant: None,
                    how: CastHow::default(),
                })
            })
            .collect()
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(crunch(false)),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(crunch(true)),
            ..Script::default()
        },
    }
}

// S #4.6 The Big Crunch — SPEC §8.10 row S 4.6, BUILD M12 row S 4.6: "casts Core 95, C+ 73, M 95, S
// 150 and S 151 once each, in that order, each rolling one effect named to both players; the nested
// casts stop at the cap; radiant each is Radiant and rolls three" (R1870006).
#[cfg(test)]
mod tests {
    use super::EDITIONS;
    use jackioh_engine::config::CALL_TO_CHAOS_CHAIN_CAP;
    use jackioh_engine::testkit::*;

    use crate::js;
    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CRUNCH: &str = "stinkyscape-004-6";
    const FILLER: &str = "core-005"; // (1) Spell Stockpile.
    /// The most actions one play's prompts may take before the test gives up on it.
    const ANSWER_BOUND: usize = 200;

    /// p1 plays The Big Crunch, its prompts left open.
    fn played(seed: &str, radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": CRUNCH, "radiant": radiant }, FILLER],
                "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER, FILLER] },
        }));
        s.play(CRUNCH, json!({}));
        s
    }

    /// p1 plays The Big Crunch, and every prompt it opens is answered.
    fn crunched(seed: &str, radiant: bool) -> Scenario {
        let mut s = played(seed, radiant);
        answer_everything(&mut s);
        s
    }

    /// Answers every prompt the play opens with the first action its chooser may take.
    fn answer_everything(s: &mut Scenario) {
        for _ in 0..ANSWER_BOUND {
            let Some(pending) = s.state().pending.clone() else {
                return;
            };
            if s.state().result.is_some() {
                return;
            }
            let action = legal_actions(s.state(), pending.player_id)
                .into_iter()
                .next()
                .expect("a prompt always has an answer");
            s.act(action, pending.player_id, "answer the prompt");
        }
        panic!("the play still asks after {ANSWER_BOUND} answers");
    }

    /// Every Call to Chaos played or cast since the scenario began, by def id, in order.
    fn chaos_plays(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::CardPlayed { def_id, .. } if EDITIONS.contains(&def_id.as_str()) => Some(def_id.clone()),
                _ => None,
            })
            .collect()
    }

    /// Every roll a Call to Chaos announced, by def id and effects.
    fn rolls(s: &Scenario) -> Vec<(String, Vec<String>)> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::ChaosRolled { def_id, effects, .. } => Some((def_id.clone(), effects.clone())),
                _ => None,
            })
            .collect()
    }

    /// A seed whose play asks nothing and rolls no nested Call to Chaos, so the five casts are the only
    /// ones and the play is the last action.
    fn plain_seed(prefix: &str, radiant: bool) -> String {
        seed_where(prefix, |seed| {
            let s = played(seed, radiant);
            s.state().pending.is_none() && s.state().result.is_none() && chaos_plays(&s).len() == EDITIONS.len()
        })
    }

    #[test]
    fn r2200004_while_training_grounds_is_held_the_crunch_casts_the_five_editions_only() {
        crate::register_all();
        assert_eq!(super::editions(), EDITIONS.to_vec());
        for def_id in super::HELD_EDITIONS {
            assert!(!set_ships(crate::card_def(def_id).set), "{def_id} is held");
        }
    }

    mod r1870006_the_big_crunch {
        use super::*;

        #[test]
        fn r1870006_casts_the_five_editions_once_each_in_order() {
            let s = played(&plain_seed("big-crunch-order", false), false);
            assert_eq!(chaos_plays(&s), EDITIONS.map(str::to_string).to_vec());
            // Each is its own card, landed in its caster's graveyard after it (R87).
            let graveyard: Vec<String> = s.pile(P1, "graveyard").into_iter().map(|card| card.def_id).collect();
            for def_id in EDITIONS.iter().chain([&CRUNCH]) {
                assert!(graveyard.iter().any(|held| held == def_id), "{def_id} in the graveyard");
            }
        }

        #[test]
        fn r1870006_each_rolls_one_effect_named_to_both() {
            let s = played(&plain_seed("big-crunch-named", false), false);
            let rolled = rolls(&s);
            let by: Vec<&str> = rolled.iter().map(|(def_id, _)| def_id.as_str()).collect();
            assert_eq!(by, EDITIONS.to_vec(), "each edition rolls once, in order");
            for (def_id, effects) in &rolled {
                assert_eq!(effects.len(), 1, "{def_id}'s base face rolls one effect");
            }
            for viewer in [P1, P2] {
                let seen = js(&s.view(viewer))["events"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|entry| entry["type"] == "chaosRolled")
                    .count();
                assert_eq!(seen, EDITIONS.len(), "{viewer:?} reads every roll");
            }
        }

        #[test]
        fn r1870006_radiant_each_radiant_rolls_three() {
            let s = played(&plain_seed("big-crunch-radiant", true), true);
            let radiant_casts: Vec<String> = s
                .events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::CardPlayed { instance_id, def_id, .. } if EDITIONS.contains(&def_id.as_str()) => {
                        Some(instance_id.clone())
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(radiant_casts.len(), EDITIONS.len());
            for id in &radiant_casts {
                assert!(s.card(id.as_str()).radiant, "each cast is Radiant");
            }
            let rolled = rolls(&s);
            assert_eq!(rolled.len(), EDITIONS.len());
            for (def_id, effects) in &rolled {
                assert_eq!(effects.len(), 3, "{def_id}'s Radiant face rolls three");
            }
        }

        #[test]
        fn r1870006_base_many_seeds_end_within_the_caps() {
            let mut nested = false;
            for i in 0..20 {
                let s = crunched(&format!("big-crunch-base-caps-{i}"), false);
                assert!(
                    s.state().pending.is_none() || s.state().result.is_some(),
                    "seed {i}: the play completes"
                );
                let plays = chaos_plays(&s).len();
                assert!(
                    plays <= EDITIONS.len() * usize::try_from(CALL_TO_CHAOS_CHAIN_CAP).unwrap_or(usize::MAX),
                    "seed {i}: {plays} Calls, past every chain's cap"
                );
                nested |= plays > EDITIONS.len();
            }
            assert!(
                nested,
                "some seed rolled a nested Call to Chaos, so a cap was in play"
            );
        }

        #[test]
        fn r1870006_radiant_many_seeds_end_within_the_caps() {
            for i in 0..20 {
                let s = crunched(&format!("big-crunch-caps-{i}"), true);
                assert!(
                    s.state().pending.is_none() || s.state().result.is_some(),
                    "seed {i}: the play completes"
                );
                let plays = chaos_plays(&s).len();
                assert!(
                    plays <= EDITIONS.len() * usize::try_from(CALL_TO_CHAOS_CHAIN_CAP).unwrap_or(usize::MAX),
                    "seed {i}: {plays} Calls, past every chain's cap"
                );
            }
        }
    }
}
