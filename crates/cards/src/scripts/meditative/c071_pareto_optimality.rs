//! Meditative #71, Pareto Optimality (docs/meditative-set.md M6): a (3) Field Spell, Mythic, the card face of ME-PARETO
//! (MD-D28–MD-D30, R1125, R1126).
//!
//! Cry: summon The Cane (`meditative-071-1`), remembered as `"cane"` (`summon.rememberAs`). While
//! this listens, each opponent play is judged against their other playable cards (`pareto.watching`);
//! a `cardResolved` for one that was not optimal sends the remembered Cane at a random enemy, and it
//! returns home after that combat's state check. Radiant: summon a Radiant Cane, and an opponent's
//! emote sets the Cane off the same way (MD-D29). The judgement stays out of the view (§10.8).

use jackioh_engine::prelude::*;
use serde_json::json;
use std::sync::Arc;

pub const ID: &str = "meditative-071";
pub const CANE: &str = "meditative-071-1";

/// The remembered Cane's attack and its way home, or nothing when no Cane is remembered.
fn cane_effects(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(cane) = recalled(ctx, "cane").and_then(|found| found.as_str().map(str::to_string)) else {
        return Vec::new();
    };
    vec![
        subsystems::cane_strike(json_as(json!({ "cane": cane }))),
        subsystems::cane_return(json_as(json!({ "cane": cane }))),
    ]
}

/// An opponent's play the judge scored below their best, on this turn.
fn non_optimal_play(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::CardResolved {
        player, instance_id, ..
    } = event
    else {
        return false;
    };
    if *player != opponent_of(ctx.controller) {
        return false;
    }
    let Some(judgement) = subsystems::pareto::judgement_of(ctx.state) else {
        return false;
    };
    !judgement.optimal && judgement.instance_id == *instance_id && judgement.turn == ctx.state.turn
}

/// An opponent's emote.
fn opponent_emoted(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::Emoted { player, .. } = event else {
        return false;
    };
    *player == opponent_of(ctx.controller)
}

/// A Field Spell's trigger: the engine consults `when` for traps alone (R99, `traps.rs`), so the run
/// checks the same predicate and answers nothing when it does not hold.
fn answering(
    id: &str,
    on: GameEventType,
    predicate: fn(&mut EffectContext<'_>, &GameEvent) -> bool,
) -> TriggerDef {
    let when: TriggerWhen = Arc::new(predicate);
    TriggerDef {
        id: id.to_string(),
        on: vec![on],
        when: Some(when),
        run: Arc::new(move |ctx: &mut EffectContext<'_>, event: &GameEvent| {
            if predicate(ctx, event) {
                cane_effects(ctx)
            } else {
                Vec::new()
            }
        }),
    }
}

fn pareto_optimality(radiant_cane: bool, hears_emotes: bool) -> Script {
    let mut triggers = vec![answering(
        "paretoOptimality",
        GameEventType::CardResolved,
        non_optimal_play,
    )];
    if hears_emotes {
        triggers.push(answering("paretoEmote", GameEventType::Emoted, opponent_emoted));
    }
    Script {
        cry: Some(hook(move |_ctx| {
            vec![summon(json_as(json!({
                "defId": CANE,
                "radiant": radiant_cane,
                "rememberAs": "cane",
            })))]
        })),
        static_flags: Some(StaticFlags {
            judges_plays: Some(true),
            hears_emotes: if hears_emotes { Some(true) } else { None },
            ..StaticFlags::default()
        }),
        triggers,
        ..Script::default()
    }
}

pub fn base() -> Script {
    pareto_optimality(false, false)
}

pub fn radiant() -> Script {
    pareto_optimality(true, true)
}

pub fn script() -> CardScripts {
    CardScripts {
        base: pareto_optimality(false, false),
        radiant: pareto_optimality(true, true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A good play and a bad one at the same price: a 4/4 for (1) and a 1/1 for (1), so the judge's
    /// best-scored card is the Vanilla.
    const GOOD: &str = "core-008";
    const BAD: &str = "core-t-felinor";

    /// The Cane `pareto` remembers.
    fn remembered_cane(s: &Scenario, pareto: &str) -> Option<String> {
        s.card(pareto)
            .memory
            .get("cane")
            .and_then(|found| found.as_str().map(str::to_string))
    }

    /// The last step's event types.
    fn last_kinds(s: &Scenario) -> Vec<&'static str> {
        s.last_events()
            .iter()
            .map(|event| event.event_type().as_str())
            .collect()
    }

    /// p1 plays Pareto (Radiant when asked) on its own turn, so its Cry summons the Cane, and hands
    /// the turn over. p2 holds the good play and the bad one, with a Scarab on its field.
    fn pareto_game(radiant: bool) -> Scenario {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "mana": 10, "hand": [{ "def": ID, "radiant": radiant }] },
            "p2": { "mana": 10, "hand": [GOOD, BAD], "field": ["core-007"] },
        }));
        s.play(ID, json!({}));
        s.end_turn();
        s
    }

    #[test]
    fn cry_summons_remembers_cane() {
        crate::register_all();
        // p2's Scarab gives p2 something to do, so its turn does not end by itself (R82) and hand
        // the turn straight back.
        let mut s = scenario(json!({
            "p1": { "mana": 10, "hand": [ID] },
            "p2": { "field": ["core-007"] },
        }));
        s.play(ID, json!({}));
        // The Cane went to the leftmost open backrow zone behind Pareto (lane 2), remembered under
        // "cane". It is Animated on your turn and entered on its controller's turn, so it animated
        // as it entered (R383): it stands in the unit zone of its lane.
        let cane_id = remembered_cane(&s, ID).expect("Pareto remembers its Cane");
        let cane = s.card(&cane_id);
        assert_eq!(cane.def_id, CANE);
        assert!(!cane.radiant);
        assert!(
            matches!(
                cane.zone,
                Zone::Field {
                    player: PlayerId::P1,
                    row: Row::Units,
                    lane: 2
                }
            ),
            "the Cane animated into its lane's unit zone, not {:?}",
            cane.zone
        );
        // At its controller's cleanup it goes home, to the backrow zone it was summoned into.
        s.end_turn();
        assert!(
            matches!(
                s.card(&cane_id).zone,
                Zone::Field {
                    player: PlayerId::P1,
                    row: Row::Backrow,
                    lane: 2
                }
            ),
            "the Cane went home to backrow lane 2, not {:?}",
            s.card(&cane_id).zone
        );
    }

    #[test]
    fn non_optimal_sends_cane() {
        let mut s = pareto_game(false);
        let cane = remembered_cane(&s, ID).expect("Pareto remembers its Cane");
        // The 1/1 under the 4/4 at the same price: the worse-scored play sends the Cane, which
        // animates for the one forced attack and goes home after it, it being p2's turn.
        s.play(BAD, json!({}));
        s.expect_events(json!([
            "cardResolved",
            "animated",
            "attackDeclared",
            "deanimated"
        ]));
        let forced = s.last_events().iter().any(|event| {
            matches!(event, GameEvent::AttackDeclared { attacker_id, forced: true, .. } if *attacker_id == cane)
        });
        assert!(forced, "the Cane made a forced attack: {:?}", last_kinds(&s));
        assert_eq!(s.backrow(P1, 2).map(|card| card.id), Some(cane));
    }

    #[test]
    fn optimal_or_own_play_does_not() {
        let mut s = pareto_game(false);
        // The best-scored play sends nothing.
        s.play(GOOD, json!({}));
        assert!(
            !last_kinds(&s).contains(&"attackDeclared"),
            "no Cane attack: {:?}",
            last_kinds(&s)
        );

        // Pareto's own controller is never judged: a bad play of p1's stores no verdict.
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "mana": 10, "hand": [ID, GOOD, BAD] } }));
        s.play(ID, json!({}));
        s.play(BAD, json!({}));
        assert_eq!(s.state().play_judgement, None);
        assert!(!last_kinds(&s).contains(&"attackDeclared"));
    }

    #[test]
    fn no_cane_no_attack() {
        // Pareto played into the last open backrow zone has no room to summon: it holds no Cane, so
        // even a bad play sends nothing. p1's Vanilla keeps p1's turn from ending by itself (R82).
        crate::register_all();
        let mut s = scenario(json!({
            "p1": {
                "mana": 10,
                "hand": [ID],
                "field": [GOOD],
                "backrow": ["core-075", "core-075", "core-075", "core-075"],
            },
            "p2": { "mana": 10, "hand": [GOOD, BAD], "field": ["core-007"] },
        }));
        s.play(ID, json!({}));
        assert_eq!(remembered_cane(&s, ID), None);
        assert!(
            !s.last_events()
                .iter()
                .any(|event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == CANE)),
            "no Cane was summoned"
        );
        s.end_turn();
        s.play(BAD, json!({}));
        assert!(
            !last_kinds(&s).contains(&"attackDeclared"),
            "no Cane, no attack: {:?}",
            last_kinds(&s)
        );
    }

    #[test]
    fn radiant_cane_emote_sends_it() {
        let mut s = pareto_game(true);
        // The Radiant face summoned a Radiant Cane.
        let cane = remembered_cane(&s, ID).expect("Pareto remembers its Cane");
        assert!(s.card(&cane).radiant);
        // The opponent's emote is an action while Pareto hears it, and it sets the Cane off.
        s.act(
            ActionBody::Emote {
                emote: EmoteId::Greetings,
            },
            P2,
            "greetings",
        );
        s.expect_events(json!(["emoted", "attackDeclared"]));
        assert!(last_kinds(&s).contains(&"attackDeclared"));
        // Pareto's own controller emoting is no action at all: its own card does not hear it.
        assert!(
            !jackioh_engine::reduce::legal_actions(s.state(), P1)
                .iter()
                .any(|body| matches!(body, ActionBody::Emote { .. }))
        );
    }

    /// Counterspell (Classic #17): "Reveals when your opponent plays a Spell: Counter it."
    const COUNTERSPELL: &str = "classic-017";
    /// Stockpile (Core #5): a (1) Spell.
    const SPELL: &str = "core-005";

    fn attacked(s: &Scenario) -> bool {
        last_kinds(s).contains(&"attackDeclared")
    }

    /// Apply `effects` as `controller`'s, outside any card.
    fn run_as(s: &mut Scenario, effects: &[Effect], controller: PlayerId) -> Vec<GameEvent> {
        use jackioh_engine::resolve::{HookOptions, apply_effects, make_context};
        use jackioh_engine::rng::Rng;
        use jackioh_engine::script::EngineSink;
        let mut events = Vec::new();
        let (seed, cursor) = (s.state().seed.clone(), s.state().rng_cursor);
        let mut rng = Rng::new(&seed, cursor);
        {
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(controller),
                    ..HookOptions::default()
                },
            );
            apply_effects(effects, &mut ctx);
        }
        s.state_mut().rng_cursor = rng.cursor();
        events
    }

    #[test]
    fn a_play_tied_with_the_best_does_nothing() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "mana": 10, "hand": [ID] },
            "p2": { "mana": 10, "hand": [GOOD, GOOD, BAD], "field": ["core-007"] },
        }));
        s.play(ID, json!({}));
        s.end_turn();
        // Two 4/4s for (1) score the same: playing either is optimal (ties count, MD-D28).
        s.play(GOOD, json!({}));
        assert!(!attacked(&s), "a tie sends nothing: {:?}", last_kinds(&s));
    }

    #[test]
    fn a_cast_is_not_judged() {
        let mut s = pareto_game(false);
        let events = run_as(
            &mut s,
            &[cast_new(CastNewArgs {
                def: BAD.into(),
                radiant: None,
                how: CastHow::default(),
            })],
            P2,
        );
        let kinds: Vec<&str> = events.iter().map(|event| event.event_type().as_str()).collect();
        assert!(
            !kinds.contains(&"attackDeclared"),
            "a cast sends nothing: {kinds:?}"
        );
    }

    #[test]
    fn a_countered_play_is_not_judged() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "mana": 10, "hand": [ID], "backrow": [{ "def": COUNTERSPELL, "faceDown": true }] },
            "p2": { "mana": 10, "hand": [GOOD, SPELL], "field": ["core-007"] },
        }));
        s.play(ID, json!({}));
        s.end_turn();
        let spell = s
            .hand(P2)
            .into_iter()
            .find(|card| card.def_id == SPELL)
            .expect("the Spell")
            .id;
        assert!(
            !subsystems::pareto::judge_play(s.state(), P2, &spell),
            "the Spell scores below the 4/4, so only the counter keeps the Cane home"
        );
        s.play(&spell, json!({}));
        assert!(last_kinds(&s).contains(&"countered"), "{:?}", last_kinds(&s));
        assert!(
            !attacked(&s),
            "a countered play sends nothing: {:?}",
            last_kinds(&s)
        );
    }

    #[test]
    fn the_judgement_survives_a_json_round_trip_and_replays_the_same() {
        let mut s = pareto_game(false);
        let round: GameState = serde_json::from_value(serde_json::to_value(s.state()).expect("serialises"))
            .expect("deserialises");
        s.play(BAD, json!({}));
        let first = jackioh_engine::replay::hash_state(s.state());
        assert!(attacked(&s));
        *s.state_mut() = round;
        s.play(BAD, json!({}));
        assert!(attacked(&s));
        assert_eq!(jackioh_engine::replay::hash_state(s.state()), first);
    }

    #[test]
    fn with_the_cane_gone_nothing_attacks() {
        let mut s = pareto_game(false);
        let cane = remembered_cane(&s, ID).expect("Pareto remembers its Cane");
        run_as(
            &mut s,
            &[exile(json_as(
                json!({ "target": { "of": "instance", "instanceId": cane } }),
            ))],
            P1,
        );
        s.expect_in_zone(&cane, "exile");
        s.play(BAD, json!({}));
        assert!(!attacked(&s), "no Cane, no attack: {:?}", last_kinds(&s));
    }

    #[test]
    fn without_a_card_that_hears_it_an_emote_is_refused() {
        let mut s = pareto_game(false);
        assert!(
            !jackioh_engine::reduce::legal_actions(s.state(), P2)
                .iter()
                .any(|body| matches!(body, ActionBody::Emote { .. }))
        );
        s.expect_refused(|s| {
            s.act(
                ActionBody::Emote {
                    emote: EmoteId::Greetings,
                },
                P2,
                "greetings",
            )
        });
    }

    #[test]
    fn judge_rates_the_better_body_optimal() {
        // The judge behind the two plays above: from p2's own view, the 4/4 for (1) is the top
        // score and the 1/1 for (1) is not, so only the second sends the Cane.
        let s = pareto_game(false);
        let hand = s.hand(P2);
        let id_of = |def_id: &str| {
            hand.iter()
                .find(|card| card.def_id == def_id)
                .map(|card| card.id.clone())
                .unwrap_or_else(|| panic!("p2 holds {def_id}"))
        };
        assert!(subsystems::pareto::judge_play(s.state(), P2, &id_of(GOOD)));
        assert!(!subsystems::pareto::judge_play(s.state(), P2, &id_of(BAD)));
    }
}
