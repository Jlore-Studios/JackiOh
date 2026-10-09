//! Meditative #71, Pareto Optimality (docs/meditative-set.md M6): the card face of ME-PARETO
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

fn pareto_optimality(radiant_cane: bool, hears_emotes: bool) -> Script {
    let play_when: TriggerWhen = Arc::new(non_optimal_play);
    let mut triggers = vec![TriggerDef {
        id: "paretoOptimality".to_string(),
        on: vec![GameEventType::CardResolved],
        when: Some(play_when),
        run: Arc::new(move |ctx: &mut EffectContext<'_>, _event: &GameEvent| cane_effects(ctx)),
    }];
    if hears_emotes {
        let emote_when: TriggerWhen = Arc::new(opponent_emoted);
        triggers.push(TriggerDef {
            id: "paretoEmote".to_string(),
            on: vec![GameEventType::Emoted],
            when: Some(emote_when),
            run: Arc::new(move |ctx: &mut EffectContext<'_>, _event: &GameEvent| cane_effects(ctx)),
        });
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

    /// `SideSetup.backrow` takes no `radiant` flag: flip the instance directly.
    fn make_radiant(s: &mut Scenario, card: &str) {
        let id = s.card(card).id.clone();
        match find_instance_mut(s.state_mut(), &id) {
            Some(instance) => instance.radiant = true,
            None => panic!("no instance {id}"),
        }
    }

    /// Pareto behind an empty board; the opponent holds a bad play and a good one.
    fn pareto_game() -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": { "backrow": [ID] },
            "p2": { "mana": 10, "hand": ["core-008", "core-012"], "field": ["core-007"] },
        }))
    }

    #[test]
    fn cry_summons_remembers_cane() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "mana": 10, "hand": [ID] } }));
        s.play(ID, json!({}));
        // The Cane stands in the leftmost open backrow zone, remembered under "cane".
        let remembered = s
            .card(ID)
            .memory
            .get("cane")
            .and_then(|found| found.as_str().map(str::to_string));
        assert!(remembered.is_some(), "Pareto remembers its Cane");
        let cane_id = remembered.unwrap();
        let cane = s.card(&cane_id);
        assert_eq!(cane.def_id, CANE);
        assert!(
            matches!(cane.zone, Zone::Field { player: PlayerId::P1, row: Row::Backrow, lane: 2 }),
            "the Cane took the leftmost open backrow zone behind Pareto, not {:?}",
            cane.zone
        );
    }

    #[test]
    fn non_optimal_sends_cane() {
        let mut s = pareto_game();
        s.end_turn();
        // core-012 (3/4 for 2) under core-008 (4/4 for 1): the worse-scored play sends the Cane.
        s.play("core-012", json!({}));
        s.expect_events(json!(["cardResolved", "attackDeclared"]));
    }

    #[test]
    fn optimal_or_own_play_does_not() {
        let mut s = pareto_game();
        s.end_turn();
        // The better-scored play sends nothing.
        s.play("core-008", json!({}));
        let seen: Vec<&str> = s.events().iter().map(|event| event.event_type().as_str()).collect();
        assert!(!seen.contains(&"attackDeclared"), "no Cane attack: {seen:?}");
    }

    #[test]
    fn no_cane_no_attack() {
        // Pareto with no room to summon holds no Cane, so even a bad play sends nothing.
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "backrow": [ID, "core-005", "core-010", "core-021", "core-023"] },
            "p2": { "mana": 10, "hand": ["core-008", "core-012"], "field": ["core-007"] },
        }));
        s.end_turn();
        s.play("core-012", json!({}));
        let seen: Vec<&str> = s.events().iter().map(|event| event.event_type().as_str()).collect();
        assert!(!seen.contains(&"attackDeclared"), "no Cane, no attack: {seen:?}");
    }

    #[test]
    fn radiant_cane_emote_sends_it() {
        let mut s = pareto_game();
        make_radiant(&mut s, ID);
        s.end_turn();
        // The opponent's emote is an action while Pareto hears it, and it sets the Cane off.
        s.act(ActionBody::Emote { emote: EmoteId::Greetings }, P2, "greetings");
        s.expect_events(json!(["emoted", "attackDeclared"]));
    }
}
