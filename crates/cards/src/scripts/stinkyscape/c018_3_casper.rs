//! S #18.3 CASPER (SPEC §8.10 row S 18.3, §7; §6.1, §6.2, §6.3, R12, R62, R78, R384, R437, R963,
//! R1045000, R1140, R1141, R1563000, R1839000, R1843000, R1866003, R1866004): (3) Field Spell, Tech,
//! Token (printed Legendary), summoned by S #18 Imitated Instrumentality.
//!   Base:    "Indestructible / Start of turn: Vote to remove you if you Floated on your last turn. /
//!            Activate: Allure a random enemy Unit."
//!   Radiant: "… / Activate: Allure a random enemy Unit and a random Unit in your opponent's hand."
//!
//! Engine: Indestructible is printed in `catalog.json` (§6.1).
//! - **The Float it saw (R1866003):** a trigger on its controller's `turnEnded` remembers that turn's
//!   player and Float (§6.2, R1563000: the unspent mana the event carries), so it reads the last turn
//!   end of its controller's it stood on the field for; leaving the field forgets it (R78).
//! - **The vote:** start of turn (R62), `vote_out` when that remembered turn end is its controller's and
//!   held a Float (R1843000). One that saw none of their turns end does not vote.
//! - **The Allure (R1866004):** Activate (R384, once a turn): one random enemy Unit acting on the field
//!   (`active_units_of`, never a carried one, R446, or a Dormant one, R1839000), Allured as K-Pop
//!   Fanatic's is (R963, R1045000): Allure's pink mark (R437), stolen at the start of its controller's
//!   next turn, destroyed with no room, nothing if it has left the field (R174). With none, nothing.
//! - **Radiant:** then one random Unit card of the opponent's hand, a hand watch with the same mark
//!   (R1140, R1141), taken into its controller's hand as theirs at the start of their next turn (R12),
//!   not taken if it has left that hand. The field's pick is drawn first. Declares no numbers.

use jackioh_engine::effects::remember;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-018-3";

/// Where CASPER keeps the last turn end of its controller's it saw: `{ player, float }`.
const LAST_TURN_KEY: &str = "lastTurnEnd";

/// The steps the delayed steals re-enter at the start of its controller's next turn (§10.6, R62).
const ALLURE_STEP: &str = "allure";
const HAND_STEP: &str = "handAllure";

/// The one thing the field steal's continuation captures: which Unit was picked.
const TARGET_KEY: &str = "targetId";

/// R437, R1045000: Allure's pink mark, on the field and in the hand alike.
fn allure_mark() -> Value {
    json!({ "mark": ALLURE_MARK, "color": ALLURE_MARK_COLOR })
}

/// R1866003: each of its controller's turn ends it sees, with the Float it held.
fn turn_watch() -> TriggerDef {
    TriggerDef::new("casper-turn-watch", &[GameEventType::TurnEnded], |ctx, event| match event {
        GameEvent::TurnEnded { player, unspent_mana, .. } if *player == ctx.controller => vec![remember(json_as(
            json!({ "key": LAST_TURN_KEY, "value": { "player": player, "float": unspent_mana } }),
        ))],
        _ => vec![],
    })
    .with_when(|ctx, event| matches!(event, GameEvent::TurnEnded { player, .. } if *player == ctx.controller))
}

/// R1866003: "votes to remove you if you Floated on your last turn".
fn vote_if_floated(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let floated = recalled(ctx, LAST_TURN_KEY).is_some_and(|seen| {
        seen["player"] == json!(ctx.controller) && seen["float"].as_i64().is_some_and(|float| float > 0)
    });
    if floated {
        vec![vote_out(Default::default())]
    } else {
        vec![]
    }
}

/// R963: the enemy Units an Allure may take, in R68's order, by id.
fn enemy_units(ctx: &EffectContext<'_>) -> Vec<String> {
    active_units_of(ctx.state, opponent_of(ctx.controller))
        .into_iter()
        .filter(|unit| !is_carried(ctx.state, unit))
        .map(|unit| unit.id.clone())
        .collect()
}

/// The Unit cards of the opponent's hand, in hand order, by id.
fn enemy_hand_units(ctx: &EffectContext<'_>) -> Vec<String> {
    zone_cards(ctx.state, opponent_of(ctx.controller), OffFieldZone::Hand)
        .into_iter()
        .filter(|card| card_type_of(ctx.state, card) == CardType::Unit)
        .map(|card| card.id)
        .collect()
}

/// R1866004: one random enemy Unit, and on the Radiant face then one random Unit card of their hand.
fn allure(ctx: &mut EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    let mut effects = Vec::new();
    let units = enemy_units(ctx);
    if let Some(target_id) = ctx.rng.pick(&units).cloned() {
        effects.push(delay(json_as(json!({
            "at": { "phase": "start", "player": "self" },
            "step": ALLURE_STEP,
            "hook": RESUME_HOOK,
            "data": { TARGET_KEY: target_id },
            // R174: a Unit that leaves the field before the steal fires is gone for good.
            "watch": target_id,
            "mark": allure_mark(),
        }))));
    }
    if radiant {
        let hand = enemy_hand_units(ctx);
        if let Some(card_id) = ctx.rng.pick(&hand).cloned() {
            effects.push(delay(json_as(json!({
                "at": { "phase": "start", "player": "self" },
                "step": HAND_STEP,
                "hook": RESUME_HOOK,
                "handWatch": [card_id],
                "mark": allure_mark(),
            }))));
        }
    }
    effects
}

/// R1045000: the picked Unit stolen, or destroyed with no open zone in its row.
fn steal_step(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(target_id) = ctx.data.get(TARGET_KEY).and_then(Value::as_str) else {
        return vec![];
    };
    vec![steal(StealTarget {
        instance_id: Some(target_id.to_string()),
        otherwise: Some(StealOtherwise::Destroy),
        ..StealTarget::default()
    })]
}

/// R12, R1140: the watched card, if still in their hand, becomes this player's, in their hand.
fn hand_step(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let ids: Vec<&str> = ctx
        .data
        .get(HAND_WATCH_KEY)
        .and_then(Value::as_array)
        .map(|ids| ids.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    if ids.is_empty() {
        return vec![];
    }
    vec![give_from_hand(json_as(json!({ "from": "enemy", "ids": ids })))]
}

fn casper(radiant: bool) -> Script {
    let activate = ActivationDecl {
        id: "allure".to_string(),
        label: if radiant {
            "Allure a random enemy Unit and a random Unit in your opponent's hand".to_string()
        } else {
            "Allure a random enemy Unit".to_string()
        },
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(move |ctx| allure(ctx, radiant)),
    };
    let mut resume = IndexMap::from([(ALLURE_STEP, hook(steal_step))]);
    if radiant {
        resume.insert(HAND_STEP, hook(hand_step));
    }
    Script {
        triggers: vec![turn_watch()],
        start_of_turn: Some(hook(vote_if_floated)),
        activations: vec![activate],
        resume,
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: casper(false),
        radiant: casper(true),
    }
}

// S #18.3 CASPER — SPEC §8.10 row S 18.3, BUILD M12 row S 18.3: "Start of turn: votes only after a turn
// of yours that ended with a Float while it stood on the field (R1866003); Activate: a random enemy Unit
// marked and stolen at the start of your next turn, destroyed with no room (R963); radiant: also a random
// Unit of their hand taken into yours at the start of your next turn, none if it left their hand"
// (R1866004).
//
// Every case crosses a turn boundary, so both sides keep a card in hand and a library: R82 ends a turn
// with nothing left in it.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, js, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CASPER: &str = "stinkyscape-018-3";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FILLER: &str = "core-016"; // Hit Job: a Spell, inert in a hand.

    /// Hand the turn over until `player` is the active one.
    fn until_active(s: &mut Scenario, player: PlayerId) {
        for _ in 0..2 {
            if s.state().active != player {
                s.end_turn();
            }
        }
        assert_eq!(s.state().active, player);
    }

    /// How many votes CASPER cast in the last step.
    fn votes(s: &Scenario) -> usize {
        s.last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::VoteCast { def_id, .. } if def_id == CASPER))
            .count()
    }

    /// CASPER (Radiant on `radiant`) in p1's backrow on p1's turn, p2 holding `hand` over `field`.
    fn table(radiant: bool, field: Value, hand: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [FILLER],
                "backrow": [{ "def": CASPER, "radiant": radiant }],
                "field": [VANILLA],
                "library": filler(4),
            },
            "p2": { "hand": hand, "field": field, "library": filler(4) },
        }))
    }

    fn allure(s: &mut Scenario) {
        let casper = s.card(CASPER).id.clone();
        s.activate(casper.as_str(), json!({ "ability": "allure" }));
    }

    /// The marks a card's view carries on the field, as K-Pop Fanatic's tests read them.
    fn marks_in(view: &PlayerView, instance_id: &str) -> Value {
        let view = js(view);
        for side in ["you", "opponent"] {
            for unit in view[side]["units"].as_array().into_iter().flatten() {
                if unit.get("instanceId").and_then(Value::as_str) == Some(instance_id) {
                    return unit.get("marks").cloned().unwrap_or(Value::Null);
                }
            }
        }
        json!("not on the field")
    }

    fn ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.id.clone()).collect()
    }

    /// The one delayed field Allure's target.
    fn allured(s: &Scenario) -> String {
        let watched: Vec<String> = s.state().delayed.iter().filter_map(|entry| entry.watch.clone()).collect();
        assert_eq!(watched.len(), 1, "one Allure on the field");
        watched[0].clone()
    }

    mod base {
        use super::*;

        /// p1's votes at its next start of turn, after a turn of its that left `mana` unspent.
        fn votes_after_a_turn_with(mana: i32) -> usize {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "backrow": [CASPER], "mana": mana, "library": filler(4) },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            s.end_turn();
            until_active(&mut s, P1);
            votes(&s)
        }

        #[test]
        fn r1866003_votes_after_a_turn_of_yours_that_ended_with_a_float() {
            assert_eq!(votes_after_a_turn_with(2), 1);
        }

        #[test]
        fn r1866003_a_turn_that_spent_everything_is_no_float() {
            // p2 Floats its whole turn's mana in between, which is not p1's Float.
            assert_eq!(votes_after_a_turn_with(0), 0);
        }

        #[test]
        fn r1866003_only_your_last_turn_counts() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "backrow": [CASPER], "mana": 2, "library": filler(4) },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            s.end_turn();
            until_active(&mut s, P1);
            assert_eq!(votes(&s), 1);
            // This turn spends everything: the next start of turn reads this one, no Float.
            s.state_mut().players.p1.mana.current = 0;
            s.end_turn();
            until_active(&mut s, P1);
            assert_eq!(votes(&s), 0);
        }

        #[test]
        fn r1866003_one_that_saw_none_of_your_turns_end_does_not_vote() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [FILLER], "backrow": [CASPER], "library": filler(4) },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            s.end_turn();
            assert_eq!(s.state().active, P1);
            assert_eq!(votes(&s), 0);
        }

        #[test]
        fn r1866004_activate_allures_a_random_enemy_unit_stolen_at_your_next_start_of_turn() {
            let mut s = table(false, json!([VANILLA, VANILLA]), json!([FILLER]));
            let theirs: Vec<String> = active_units_of(s.state(), P2).iter().map(|unit| unit.id.clone()).collect();
            allure(&mut s);
            let target = allured(&s);
            assert!(theirs.contains(&target));
            // R437: the pink mark, in both views.
            let pink = json!([{ "mark": "allure", "color": "pink" }]);
            assert_eq!(marks_in(&s.view(P1), &target), pink);
            assert_eq!(marks_in(&s.view(P2), &target), pink);
            // Once a turn.
            let casper = s.card(CASPER).id.clone();
            s.expect_refused(|s| s.activate(casper.as_str(), json!({ "ability": "allure" })));

            s.end_turn();
            assert_eq!(s.card(target.as_str()).controller, P2, "not before p1's next turn");
            until_active(&mut s, P1);
            assert!(s.state().delayed.is_empty());
            assert_eq!(s.card(target.as_str()).controller, P1);
            s.expect_in_zone(target.as_str(), "field");
        }

        #[test]
        fn r1866004_with_no_open_unit_zone_it_is_destroyed() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [FILLER],
                    "backrow": [CASPER],
                    "field": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA],
                    "library": filler(4),
                },
                "p2": { "hand": [FILLER], "field": [VANILLA], "library": filler(4) },
            }));
            allure(&mut s);
            let target = allured(&s);
            s.end_turn();
            until_active(&mut s, P1);
            s.expect_in_zone(target.as_str(), "graveyard");
            assert_eq!(s.card(target.as_str()).owner, P2);
        }

        #[test]
        fn r1866004_a_dormant_enemy_unit_is_never_chosen() {
            for seed in 0..8 {
                let mut s = scenario(json!({
                    "seed": format!("casper-dormant-{seed}"),
                    "p1": { "hand": [FILLER], "backrow": [CASPER], "library": filler(4) },
                    "p2": {
                        "hand": [FILLER],
                        "field": [{ "def": VANILLA, "slumber": { "turns": 2 } }, VANILLA],
                        "library": filler(4),
                    },
                }));
                let awake = s.unit(P2, 2).expect("the awake Unit").id;
                allure(&mut s);
                assert_eq!(allured(&s), awake);
            }
            // With only a Dormant one, nothing.
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "backrow": [CASPER], "library": filler(4) },
                "p2": { "hand": [FILLER], "field": [{ "def": VANILLA, "slumber": { "turns": 2 } }], "library": filler(4) },
            }));
            allure(&mut s);
            assert!(s.state().delayed.is_empty());
        }

        #[test]
        fn r1866004_the_base_face_never_marks_their_hand() {
            let mut s = table(false, json!([VANILLA]), json!([VANILLA, FILLER]));
            allure(&mut s);
            assert!(s.state().delayed.iter().all(|entry| entry.hand_watch.is_none()));
            assert_eq!(s.view(P1).opponent.hand_marked, None);
        }
    }

    mod radiant {
        use super::*;

        /// The one hand watch's cards.
        fn hand_watched(s: &Scenario) -> Vec<String> {
            let watches: Vec<Vec<String>> =
                s.state().delayed.iter().filter_map(|entry| entry.hand_watch.clone()).collect();
            assert_eq!(watches.len(), 1, "one Allure in their hand");
            watches[0].clone()
        }

        #[test]
        fn r1866004_radiant_also_allures_a_random_unit_card_of_their_hand() {
            let mut s = table(true, json!([VANILLA]), json!([VANILLA, FILLER]));
            let unit_card = s.hand(P2)[0].id.clone();
            allure(&mut s);
            let target = allured(&s);
            assert_eq!(hand_watched(&s), std::slice::from_ref(&unit_card));
            // R1141: the caster sees how many, never which; the owner sees the mark.
            assert_eq!(s.view(P1).opponent.hand_marked, Some(1));
            s.end_turn();
            until_active(&mut s, P1);
            assert!(s.state().delayed.is_empty());
            assert_eq!(s.card(target.as_str()).controller, P1);
            // R12: the card is p1's now, in p1's hand.
            assert!(ids(&s.hand(P1)).contains(&unit_card));
            assert_eq!(s.card(unit_card.as_str()).owner, P1);
        }

        #[test]
        fn r1866004_a_hand_unit_played_meanwhile_is_not_taken() {
            let mut s = table(true, json!([]), json!([VANILLA, FILLER]));
            let unit_card = s.hand(P2)[0].id.clone();
            allure(&mut s);
            assert_eq!(hand_watched(&s), std::slice::from_ref(&unit_card));
            s.end_turn();
            assert_eq!(s.state().active, P2);
            s.play(unit_card.as_str(), json!({}));
            assert!(s.state().delayed.is_empty());
            until_active(&mut s, P1);
            assert_eq!(s.card(unit_card.as_str()).controller, P2);
            s.expect_in_zone(unit_card.as_str(), "field");
        }

        #[test]
        fn r1866004_no_unit_card_in_their_hand_marks_none_there() {
            let mut s = table(true, json!([VANILLA]), json!([FILLER, FILLER]));
            allure(&mut s);
            allured(&s);
            assert!(s.state().delayed.iter().all(|entry| entry.hand_watch.is_none()));
            assert_eq!(s.view(P1).opponent.hand_marked, None);
        }

        #[test]
        fn r1866004_with_no_enemy_unit_on_the_field_the_hand_is_still_allured() {
            let mut s = table(true, json!([]), json!([VANILLA, FILLER]));
            allure(&mut s);
            assert_eq!(s.state().delayed.len(), 1);
            assert_eq!(hand_watched(&s).len(), 1);
        }

        #[test]
        fn r1866003_the_radiant_face_votes_after_a_float_as_the_base_does() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [FILLER],
                    "backrow": [{ "def": CASPER, "radiant": true }],
                    "mana": 2,
                    "library": filler(4),
                },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            s.end_turn();
            until_active(&mut s, P1);
            assert_eq!(votes(&s), 1);
        }
    }
}
