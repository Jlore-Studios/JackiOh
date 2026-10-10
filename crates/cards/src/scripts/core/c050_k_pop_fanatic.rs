//! #50 K-Pop Fanatic (SPEC §8.2; R13, R15, R62, R68, R76, R81, R90, R126, R127, R174, R275, R282, R437).
//!   Base:    "Cry: choose an enemy permanent; at the start of your next turn, steal it"
//!   Radiant: "Divine Shield; Cry: choose an enemy permanent; at the start of your next turn, steal
//!            it; it becomes Radiant" (R275; "Divine Shield" is printed in `catalog.json`).
//!
//! The choice is a declared target made at PLAY time (R81) over both enemy rows ("permanent", §6.3);
//! R90 validates it, a face-down trap may be named unseen (§9.1), a Stack pile offers its top (R13).
//! The delay (§8.2 Engine cell) is keyed to the TARGET, not this unit, so it fires even if K-Pop
//! Fanatic died (R76, R127); R62 and R68 order it and `run_resume` re-enters it (§10.6, R126). Its
//! fizzles (R76, R174, R15) are the engine's, not re-checked here.
//! The rider (R282) lands only on a card the steal took: `takeable` before it, `held_now` after.
//! THE MARK (R437): the pending steal's target carries a purple `steal` mark until it resolves.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-050";

/// The step `state.delayed` re-enters at the start of this player's next turn (§10.6, R62).
const STEAL_STEP: &str = "steal";

/// The one thing the continuation captures: which permanent was chosen (§8.2 Engine cell).
const TARGET_KEY: &str = "targetId";

/// R437: the pending steal, as the mark its target carries while it waits — purple.
const STEAL_MARK: StealMark = StealMark {
    mark: "steal",
    color: "purple",
};

/// The mark's two strings, written into the `delay` literal.
struct StealMark {
    mark: &'static str,
    color: &'static str,
}

impl StealMark {
    /// The `CardMark` literal `delay` takes.
    fn to_json(&self) -> Value {
        json!({ "mark": self.mark, "color": self.color })
    }
}

/// The play-time pick (R81), read straight off the context — reading state is not mutating it.
fn chosen_instance_id(ctx: &EffectContext<'_>) -> Option<String> {
    match ctx.targets.first() {
        Some(Selection::Instance { instance_id }) => Some(instance_id.clone()),
        _ => None,
    }
}

/// The captured id, narrowed rather than cast: `data` is JSON that crossed a turn boundary.
fn captured_target_id(ctx: &EffectContext<'_>) -> Option<String> {
    ctx.data.get(TARGET_KEY).and_then(Value::as_str).map(str::to_string)
}

/// R13: a card on the field on top of its pile — the only card a steal can take. A backrow zone holds
/// one card, so for a Field Spell or a Trap this is "on the field"; in a unit zone it leaves out a
/// card dormant under a Stack pile (§3.2).
fn on_top_of_its_pile(state: &GameState, card: &CardInstance) -> bool {
    match slot_of(state, card) {
        Some(at) => card_at(state, at).map(|top| top.id == card.id).unwrap_or(false),
        None => false,
    }
}

/// R282, before the steal: the card stands on the field under the other player, so it can be taken.
fn takeable(ctx: &EffectContext<'_>, id: &str) -> bool {
    let state: &GameState = &*ctx.state;
    match find_instance(state, id) {
        Some(card) => on_top_of_its_pile(state, card) && card.controller != ctx.controller,
        None => false,
    }
}

/// R282, after the steal: the card stands on the field under this player.
fn held_now(ctx: &EffectContext<'_>, id: &str) -> bool {
    let state: &GameState = &*ctx.state;
    match find_instance(state, id) {
        Some(card) => on_top_of_its_pile(state, card) && card.controller == ctx.controller,
        None => false,
    }
}

/// R62's continuation, registered once in the `resume` table (R126). R76's fizzles are steal's own
/// no-ops: a target gone from the field, or one already yours.
fn steal_step(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(target_id) = captured_target_id(ctx) else {
        return vec![];
    };
    vec![steal(json_as(json!({ "instanceId": target_id })))]
}

/// The radiant face's step: the same steal, then R282's rider for the card the steal took and nothing
/// else. `takeable` is read as the list is built, before the steal; `for_each_card` reads its set when
/// the list reaches it, after, so the set is the stolen card or nothing. The face the Cry ran is the
/// one stored with the delay, whatever the Fanatic shows when it fires.
fn radiant_steal_step(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(target_id) = captured_target_id(ctx) else {
        return vec![];
    };
    let mut effects: Vec<Effect> = vec![steal(json_as(json!({ "instanceId": target_id })))];
    if !takeable(ctx, &target_id) {
        return effects;
    }
    let rider_id = target_id.clone();
    effects.push(for_each_card(ForEachCardArgs {
        cards: Arc::new(move |now: &mut EffectContext<'_>| -> Vec<String> {
            if held_now(now, &rider_id) {
                vec![rider_id.clone()]
            } else {
                vec![]
            }
        }),
        each: Arc::new(|instance_id: &str| -> Effect {
            set_radiant(json_as(json!({ "instanceId": instance_id })))
        }),
    }));
    effects
}

/// The Cry both faces share: choose the enemy permanent and arm the steal (§8.2 Engine cell).
fn kpop_fanatic(step: Hook) -> Script {
    Script {
        // §6.3: a permanent is a Unit, Field Spell, Trap or Field Trap, so both enemy rows are offered.
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "enemy", "of": ["unit", "backrow"] }),
        )],
        cry: Some(hook(|ctx| {
            let Some(target_id) = chosen_instance_id(ctx) else {
                return vec![];
            };
            vec![delay(json_as(json!({
                "at": { "phase": "start", "player": "self" },
                "step": STEAL_STEP,
                "hook": RESUME_HOOK,
                "data": { TARGET_KEY: target_id },
                // R174: a target that leaves the field before the steal fires is gone for good, even if the
                // same card is back by then (bounced and replayed, or a Reborn body) — R76's fizzle.
                "watch": target_id,
                // R437: and it carries the steal's mark in both views while it waits.
                "mark": STEAL_MARK.to_json(),
            })))]
        })),
        resume: IndexMap::from([(STEAL_STEP, step)]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: kpop_fanatic(hook(steal_step)),
        // "Divine Shield" is printed in the catalog; the step adds R282's rider.
        radiant: kpop_fanatic(hook(radiant_steal_step)),
    }
}

// #50 K-Pop Fanatic — SPEC §8.2, BUILD M4-T4: "Steal fires at your next start of turn even if it
// died (R76); fizzles if the target left; radiant Divine Shield". R437: the pending steal marks its
// target purple in both views while it waits. A base #50 made Radiant on the field gains its radiant
// face's Divine Shield at once, even after a granted one was spent (§5.2).
//
// Radiant (R275, R282): the rider lands only on a card the delayed steal took, never on a target that
// left the field (R76, R174), lies dormant under a Stack pile (R13), was already this player's, or
// stayed with the opponent because the row was full (R15). Each has an `R282` case below.
//
// Every case crosses a turn boundary, so both sides keep a card in hand, a unit and library cards:
// the engine auto-ends a turn with nothing left (R82) and an empty library adds fatigue. Two
// `end_turn()`s return to this player's own start of turn, where `turn::run_delayed` resolves the
// entry before any start-of-turn trigger fires (R62).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const KPOP: &str = "core-050";
    const SEVEN_SEVEN: &str = "core-025";
    const DUELIST: &str = "core-045";
    const ROCK: &str = "core-066";
    const FILLER: &str = "core-016";
    const MENACE: &str = "core-019";
    const LIBRARY: [&str; 3] = [SEVEN_SEVEN, "core-008", "core-020"];

    /// The card's two faces.
    fn faces() -> CardScripts {
        super::script()
    }

    /// A `{ pick: "instance", instanceId }` target list, the play action's declared pick (R81).
    fn target(id: &str) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": id }] })
    }

    /// The unit on `player`'s lane, copied, or the setup's failure message.
    fn unit_at(g: &Scenario, player: PlayerId, lane: i32, what: &str) -> CardInstance {
        g.unit(player, lane).unwrap_or_else(|| panic!("{what}")).clone()
    }

    /// The id of the unit on `player`'s lane, if any.
    fn unit_id(g: &Scenario, player: PlayerId, lane: i32) -> Option<String> {
        g.unit(player, lane).map(|unit| unit.id.clone())
    }

    /// The id of the backrow card on `player`'s lane, if any.
    fn backrow_id(g: &Scenario, player: PlayerId, lane: i32) -> Option<String> {
        g.backrow(player, lane).map(|card| card.id.clone())
    }

    /// How many events of this type the whole scenario has seen.
    fn count_events(g: &Scenario, kind: GameEventType) -> usize {
        g.events().iter().filter(|event| event.event_type() == kind).count()
    }

    /// Hand the turn over until `player` is the active one (R82 can end a turn with nothing left in it).
    fn until_active(g: &mut Scenario, player: PlayerId) {
        if g.state().active != player {
            g.end_turn();
        }
        if g.state().active != player {
            g.end_turn();
        }
        assert_eq!(g.state().active, player);
    }

    /// Whether any Make Radiant cue named this card (§10.3: every visible change is an event).
    fn radiant_set_on(g: &Scenario, instance_id: &str) -> bool {
        g.events().iter().any(|event| {
            matches!(event, GameEvent::RadiantSet { instance_id: id, .. } if id == instance_id)
        })
    }

    const FLOOD: &str = "core-017";
    const CLONE_MACHINE: &str = "core-033";
    const MIND_CONTROL: &str = "core-049";
    const FIENDER: &str = "core-092";
    const VANILLA: &str = "core-008";

    /// `{ def: KPOP, radiant: true }`: the radiant face in hand (§5.2).
    fn radiant_kpop() -> Value {
        json!({ "def": KPOP, "radiant": true })
    }

    const SAINTESS: &str = "core-081";
    const SURGERY: &str = "core-063";

    // R437: the pending steal marks its target purple, in both views, while it waits.

    fn steal_mark() -> Value {
        json!({ "mark": "steal", "color": "purple" })
    }

    /// A `marked` event's JSON with the steal mark spread in.
    fn marked(instance_id: &str, added: bool) -> Value {
        json!({ "type": "marked", "instanceId": instance_id, "mark": "steal", "color": "purple", "added": added })
    }

    /// The marks a card's view carries, wherever the viewer sees it on the field: the marks JSON, `Null`
    /// for a card on the field with none, or `"not on the field"`.
    fn marks_in(view: &PlayerView, instance_id: &str) -> Value {
        let view = serde_json::to_value(view).expect("a view serialises");
        for side in ["you", "opponent"] {
            for unit in view[side]["units"].as_array().into_iter().flatten() {
                if unit.get("instanceId").and_then(Value::as_str) == Some(instance_id) {
                    return unit.get("marks").cloned().unwrap_or(Value::Null);
                }
            }
            for card in view[side]["backrow"].as_array().into_iter().flatten() {
                if !card.is_null() && card.get("instanceId").and_then(Value::as_str) == Some(instance_id) {
                    return card.get("marks").cloned().unwrap_or(Value::Null);
                }
            }
        }
        json!("not on the field")
    }

    /// The `marked` events among these, as JSON.
    fn mark_events(events: &[GameEvent]) -> Vec<Value> {
        events
            .iter()
            .filter(|event| event.event_type() == GameEventType::Marked)
            .map(|event| serde_json::to_value(event).expect("an event serialises"))
            .collect()
    }

    mod n50_k_pop_fanatic {
        use super::*;

        #[test]
        fn r81_s8_2_engine_base_the_cry_schedules_a_delayed_effect_keyed_to_the_chosen_permanent() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "hand": [KPOP, FILLER], "library": LIBRARY },
                "p2": { "hand": [FILLER], "field": [{ "def": SEVEN_SEVEN, "lane": 4 }], "library": LIBRARY },
            }));
            let prey = unit_at(&g, P2, 4, "setup: p2 should hold the 7/7 in lane 4");

            g.play(KPOP, target(&prey.id));

            // The choice was made at play time, so resolution never paused (R81, §10.6).
            assert!(g.state().pending.is_none());
            // Nothing is stolen on the turn it is played.
            assert_eq!(unit_id(&g, P2, 4), Some(prey.id.clone()));
            assert_eq!(g.card(&prey).controller, P2);

            assert_eq!(g.state().delayed.len(), 1);
            let entry = g.state().delayed[0].clone();
            assert_eq!(entry.owner, P1);
            assert_eq!(
                serde_json::to_value(entry.at).unwrap(),
                json!({ "phase": "start", "player": "p1" })
            );
            assert_eq!(entry.resume.def_id, KPOP);
            assert_eq!(entry.resume.step, "steal");
            // Keyed to the TARGET, which is what lets it outlive K-Pop Fanatic (R76).
            assert_eq!(entry.resume.data.get("targetId"), Some(&json!(prey.id)));
        }

        #[test]
        fn r62_r15_base_the_steal_happens_at_your_next_start_of_turn_same_lane_if_free() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "hand": [KPOP, FILLER], "library": LIBRARY },
                "p2": { "hand": [FILLER], "field": [{ "def": SEVEN_SEVEN, "lane": 2 }], "library": LIBRARY },
            }));
            let prey = unit_at(&g, P2, 2, "setup: p2 should hold the 7/7 in lane 2");
            g.play(KPOP, target(&prey.id));

            g.end_turn();
            // The entry names p1's start of turn, so the opponent's start of turn does not fire it.
            assert_eq!(g.state().active, P2);
            assert_eq!(g.state().delayed.len(), 1);
            assert_eq!(unit_id(&g, P2, 2), Some(prey.id.clone()));

            g.end_turn();

            assert_eq!(g.state().active, P1);
            assert_eq!(unit_id(&g, P1, 2), Some(prey.id.clone()));
            assert!(g.unit(P2, 2).is_none());
            // R12: control moved, ownership did not.
            assert_eq!(g.card(&prey).controller, P1);
            assert_eq!(g.card(&prey).owner, P2);
            // The base face has no rider: the stolen card keeps its face (R282 is the radiant face's).
            assert!(!g.card(&prey).radiant);
            assert_eq!(count_events(&g, GameEventType::RadiantSet), 0);
            // The entry is spent, so it never fires twice.
            assert!(g.state().delayed.is_empty());
            g.expect_events(json!(["cardPlayed", "turnStarted", "controlChanged"]));
        }

        #[test]
        fn r15_base_the_first_free_zone_when_your_same_lane_is_occupied() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "hand": [KPOP, FILLER], "field": [{ "def": ROCK, "lane": 2 }], "library": LIBRARY },
                "p2": { "hand": [FILLER], "field": [{ "def": SEVEN_SEVEN, "lane": 2 }], "library": LIBRARY },
            }));
            let prey = unit_at(&g, P2, 2, "setup: p2 should hold the 7/7 in lane 2");
            // K-Pop Fanatic takes the leftmost free zone, lane 1 (R64).
            g.play(KPOP, target(&prey.id));
            assert_eq!(g.unit(P1, 1).map(|unit| unit.def_id.clone()), Some(KPOP.to_string()));

            g.end_turn();
            g.end_turn();

            // Lanes 1 and 2 are taken, so `first_free_zone` lands it in lane 3.
            assert_eq!(unit_id(&g, P1, 3), Some(prey.id.clone()));
        }

        #[test]
        fn r76_base_the_steal_fires_even_though_k_pop_fanatic_died_in_between() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "hand": [KPOP, FILLER], "library": LIBRARY },
                // The 7/7 carries 2 damage into the case. It cannot pick damage up in combat here: §8 row 25
                // prints Armor 7, so the 1 a 1/1 strikes back with is 0 after Armor and is not a damage
                // instance at all (§4.4 step 2, R63). Seeding it is the only way to have damage on the prey
                // before the steal, which is what makes the R78 claim below observable.
                "p2": {
                    "hand": [FILLER],
                    "field": [{ "def": SEVEN_SEVEN, "lane": 2, "damage": 2 }],
                    "library": LIBRARY,
                },
            }));
            let prey = unit_at(&g, P2, 2, "setup: p2 should hold the 7/7 in lane 2");
            g.play(KPOP, target(&prey.id));
            let fanatic = unit_at(&g, P1, 1, "K-Pop Fanatic should be in p1's lane 1");

            g.end_turn();
            // The 7/7 eats the 1/1 on the opponent's turn; it takes 1 back and stays on the field.
            g.attack(&prey, &fanatic);
            g.expect_in_zone(&fanatic, "graveyard");
            assert_eq!(g.state().delayed.len(), 1);

            g.end_turn();

            // R76: the continuation lives in `state.delayed`, not on the unit.
            assert_eq!(unit_id(&g, P1, 2), Some(prey.id.clone()));
            assert_eq!(g.card(&prey).controller, P1);
            // A steal moves `controller` and nothing else: the card never leaves the field, so R78's reset
            // — which is about LEAVING it — does not run and the damage it was carrying is still there.
            assert_eq!(g.card(&prey).damage, 2);
        }

        #[test]
        fn r76_base_it_fizzles_when_the_target_left_the_field() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "hand": [KPOP, FILLER], "field": [{ "def": ROCK, "lane": 5 }], "library": LIBRARY },
                "p2": { "hand": [FILLER], "field": [{ "def": DUELIST, "lane": 2 }], "library": LIBRARY },
            }));
            let prey = unit_at(&g, P2, 2, "setup: p2 should hold Deft Duelist in lane 2");
            g.play(KPOP, target(&prey.id));

            g.end_turn();
            // The 4/3 throws itself at a 10/10 Indestructible wall and dies.
            g.attack(&prey, "core-066");
            g.expect_in_zone(&prey, "graveyard");

            g.end_turn();

            // `steal` no-ops off the field (control means nothing there, R12), so nothing is taken.
            assert!(g.pile(P2, "graveyard").iter().any(|card| card.id == prey.id));
            assert!(g.unit(P1, 2).is_none());
            assert!(g.state().delayed.is_empty());
            assert_eq!(count_events(&g, GameEventType::ControlChanged), 0);
        }

        #[test]
        fn r76_base_it_fizzles_when_the_target_is_already_under_your_control() {
            crate::register_all();
            let mut g = scenario(json!({
                // K-Pop Fanatic costs 1 and Snom Bunny Mind Control 4.
                "p1": { "hand": [KPOP, "core-049"], "library": LIBRARY, "mana": 5 },
                "p2": { "hand": [FILLER], "field": [{ "def": SEVEN_SEVEN, "lane": 2 }], "library": LIBRARY },
            }));
            let prey = unit_at(&g, P2, 2, "setup: p2 should hold the 7/7 in lane 2");

            g.play(KPOP, target(&prey.id));
            // #49 takes the same permanent this turn, for the 4 mana left.
            g.play("core-049", target(&prey.id));
            assert_eq!(g.card(&prey).controller, P1);

            g.end_turn();
            g.end_turn();

            assert_eq!(g.card(&prey).controller, P1);
            assert!(g.state().delayed.is_empty());
            // One steal, not two: `take_control` refuses a card this player already controls.
            assert_eq!(count_events(&g, GameEventType::ControlChanged), 1);
        }

        #[test]
        fn radiant_divine_shield_eats_the_first_hit_and_the_delayed_steal_lands_with_its_rider_r282() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "hand": [{ "def": KPOP, "radiant": true }, FILLER], "library": LIBRARY },
                "p2": { "hand": [FILLER], "field": [{ "def": DUELIST, "lane": 3 }], "library": LIBRARY },
            }));
            let prey = unit_at(&g, P2, 3, "setup: p2 should hold Deft Duelist in lane 3");

            g.play(KPOP, target(&prey.id));
            let fanatic = unit_at(&g, P1, 1, "K-Pop Fanatic should be in p1's lane 1");
            g.expect_stats(&fanatic, json!({ "attack": 2, "maxHealth": 2, "health": 2 }));
            assert!(g.stats(&fanatic).keywords.iter().any(|keyword| keyword.kind() == KeywordKind::DivineShield));

            g.end_turn();
            g.attack(&prey, &fanatic);

            // §6.1: the shield absorbs the whole hit, so a 4/3 attacker cannot kill the 2/2.
            g.expect_events(json!(["divineShieldLost"]));
            g.expect_in_zone(&fanatic, "field");
            assert_eq!(g.card(&fanatic).damage, 0);
            assert_eq!(g.card(&fanatic).divine_shield_spent, Some(true));
            // It still struck back for 2.
            assert_eq!(g.card(&prey).damage, 2);

            g.end_turn();

            assert_eq!(unit_id(&g, P1, 3), Some(prey.id.clone()));
            assert_eq!(g.card(&prey).controller, P1);
            // R282: the stolen Duelist becomes Radiant in place (R22): the 8/6 face, its 2 damage kept.
            assert!(g.card(&prey).radiant);
            g.expect_stats(&prey, json!({ "attack": 8, "maxHealth": 6, "health": 4 }));
        }

        #[test]
        fn radiant_the_steal_still_fires_after_a_radiant_k_pop_fanatic_dies_r76() {
            crate::register_all();
            // The radiant face is a 2/2 with Divine Shield, so killing it takes TWO hits — and R76 gives
            // the opponent exactly one turn in which to land them, because the steal resolves at p1's very
            // next start of turn. One attacker cannot do it (a unit has one attack exertion per turn,
            // §4.1), so p2 fields two: the Duelist pops the shield and the 7/7 finishes the job.
            let mut g = scenario(json!({
                "p1": { "hand": [{ "def": KPOP, "radiant": true }, FILLER], "library": LIBRARY },
                "p2": {
                    "hand": [FILLER],
                    "field": [
                        { "def": DUELIST, "lane": 3 },
                        { "def": SEVEN_SEVEN, "lane": 4 },
                    ],
                    "library": LIBRARY,
                },
            }));
            let prey = unit_at(&g, P2, 4, "setup: p2 should hold the 7/7 in lane 4");
            let opener = unit_at(&g, P2, 3, "setup: p2 should hold Deft Duelist in lane 3");
            g.play(KPOP, target(&prey.id));
            let fanatic = unit_at(&g, P1, 1, "K-Pop Fanatic should be in p1's lane 1");

            g.end_turn();
            // §6.1: the shield absorbs the whole first hit, so the 4/3 cannot kill the 2/2.
            g.attack(&opener, &fanatic);
            g.expect_in_zone(&fanatic, "field");
            // The second hit lands on a shieldless 2/2 and kills it, on the same turn.
            g.attack(&prey, &fanatic);
            g.expect_in_zone(&fanatic, "graveyard");

            g.end_turn();

            // R76: the delayed steal is state, not something the dead unit was holding.
            assert_eq!(unit_id(&g, P1, 4), Some(prey.id.clone()));
            assert_eq!(g.card(&prey).controller, P1);
            // R282: and so is its rider — the face that ran the Cry is stored with the entry.
            assert!(g.card(&prey).radiant);
        }

        #[test]
        fn s8_conventions_a_cry_with_nothing_to_choose_fizzles_and_the_unit_still_enters() {
            crate::register_all();
            let mut g = scenario(json!({ "p1": { "hand": [KPOP, FILLER], "library": LIBRARY } }));

            g.play(KPOP, json!({}));

            g.expect_in_zone(KPOP, "field");
            assert!(g.state().delayed.is_empty());
        }

        #[test]
        fn r81_r282_both_faces_declare_one_enemy_permanent_and_one_resume_step_only_the_step_differs() {
            crate::register_all();
            let def = crate::card_def(KPOP);
            let CardScripts { base, radiant } = faces();
            assert_eq!(def.id, KPOP);
            assert_eq!(def.type_, CardType::Unit);
            assert_eq!(def.cost, CardCost::Fixed(1));
            assert_eq!((def.base.attack, def.base.health), (Some(1), Some(1)));
            assert_eq!((def.radiant.attack, def.radiant.health), (Some(2), Some(2)));
            // "Divine Shield" without "Plus" is the radiant form's whole keyword list (§8 Conventions).
            assert_eq!(def.radiant.keywords, vec![Keyword::DivineShield]);
            assert!(def.base.keywords.is_empty());

            let decl = json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "enemy", "of": ["unit", "backrow"] } }]);
            assert_eq!(serde_json::to_value(&base.targets).unwrap(), decl);
            assert_eq!(serde_json::to_value(&radiant.targets).unwrap(), decl);
            assert!(base.modes.is_empty());
            assert!(base.cry.is_some());
            assert!(radiant.cry.is_some());
            let base_step = base.resume.get("steal").expect("the base face's steal step");
            // R282: the radiant face's step is its own — the steal plus "it becomes Radiant".
            let radiant_step = radiant.resume.get("steal").expect("the radiant face's steal step");
            assert!(!std::sync::Arc::ptr_eq(radiant_step, base_step));
        }
    }

    mod n50_k_pop_fanatic_radiant_r282_the_rider_lands_only_on_a_card_the_steal_took {
        use super::*;

        #[test]
        fn r282_the_steal_lands_and_the_stolen_unit_becomes_radiant_in_place_its_damage_kept_r22() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "hand": [radiant_kpop(), FILLER], "library": LIBRARY },
                "p2": { "hand": [FILLER], "field": [{ "def": SEVEN_SEVEN, "lane": 2, "damage": 2 }], "library": LIBRARY },
            }));
            let prey = unit_at(&g, P2, 2, "setup: p2 should hold the 7/7 in lane 2");
            g.play(KPOP, target(&prey.id));
            // Nothing becomes Radiant on the turn it is played: the rider rides the steal.
            assert!(!g.card(&prey).radiant);

            g.end_turn();
            g.end_turn();

            assert_eq!(g.state().active, P1);
            assert_eq!(unit_id(&g, P1, 2), Some(prey.id.clone()));
            assert_eq!(g.card(&prey).controller, P1);
            assert!(g.card(&prey).radiant);
            // #25's Radiant face: a 14/14 with Armor 7 and Reborn, the 2 damage still on it.
            g.expect_stats(&prey, json!({ "attack": 14, "maxHealth": 14, "health": 12 }));
            let kinds: Vec<KeywordKind> = g.stats(&prey).keywords.iter().map(|keyword| keyword.kind()).collect();
            assert_eq!(kinds, vec![KeywordKind::Armor, KeywordKind::Reborn]);
            g.expect_events(json!(["turnStarted", "controlChanged", "radiantSet"]));
        }

        #[test]
        fn r282_a_stolen_field_spell_becomes_radiant_too_a_permanent_is_either_row_s6_3() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "hand": [radiant_kpop(), FILLER], "library": LIBRARY },
                "p2": { "hand": [FILLER], "field": [VANILLA], "backrow": [{ "def": CLONE_MACHINE, "lane": 3 }], "library": LIBRARY },
            }));
            let prey = g
                .backrow(P2, 3)
                .expect("setup: p2 should hold the Clone Machine in backrow lane 3")
                .clone();
            g.play(KPOP, target(&prey.id));

            g.end_turn();
            g.end_turn();

            assert_eq!(g.state().active, P1);
            assert_eq!(backrow_id(&g, P1, 3), Some(prey.id.clone()));
            assert_eq!(g.card(&prey).controller, P1);
            assert!(g.card(&prey).radiant);
        }

        #[test]
        fn r282_a_target_that_died_before_the_steal_is_not_made_radiant_in_the_graveyard_r76() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "hand": [radiant_kpop(), FILLER], "field": [{ "def": ROCK, "lane": 5 }], "library": LIBRARY },
                "p2": { "hand": [FILLER], "field": [{ "def": DUELIST, "lane": 2 }], "library": LIBRARY },
            }));
            let prey = unit_at(&g, P2, 2, "setup: p2 should hold Deft Duelist in lane 2");
            g.play(KPOP, target(&prey.id));

            g.end_turn();
            // The 4/3 throws itself at a 10/10 Indestructible wall and dies.
            g.attack(&prey, ROCK);
            g.expect_in_zone(&prey, "graveyard");
            g.end_turn();

            assert_eq!(g.state().active, P1);
            g.expect_in_zone(&prey, "graveyard");
            assert!(!g.card(&prey).radiant);
            assert!(!radiant_set_on(&g, &prey.id));
            assert!(g.state().delayed.is_empty());
        }

        #[test]
        fn r282_a_target_bounced_to_its_controller_s_hand_is_not_made_radiant_there_where_p1_may_not_read_it_r174() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "hand": [radiant_kpop(), FILLER], "field": [{ "def": VANILLA, "lane": 1 }], "library": LIBRARY },
                "p2": { "hand": [FLOOD, FILLER], "field": [{ "def": SEVEN_SEVEN, "lane": 2 }], "library": LIBRARY },
            }));
            let prey = unit_at(&g, P2, 2, "setup: p2 should hold the 7/7 in lane 2");
            g.play(KPOP, target(&prey.id));
            g.end_turn();

            // #17 Flood bounces every unit on both sides, the prey into p2's hand.
            g.play(FLOOD, json!({}));
            g.expect_in_zone(&prey, "hand");
            until_active(&mut g, P1);

            g.expect_in_zone(&prey, "hand");
            assert_eq!(g.card(&prey).controller, P2);
            assert!(!g.card(&prey).radiant);
            assert!(!radiant_set_on(&g, &prey.id));
            assert!(g.state().delayed.is_empty());
        }

        #[test]
        fn r282_a_target_bounced_and_replayed_is_a_new_arrival_neither_stolen_nor_made_radiant_r174_r78() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "hand": [radiant_kpop(), FILLER], "field": [{ "def": VANILLA, "lane": 1 }], "library": LIBRARY },
                // The Coin pays for the replay: Flood costs 4.
                "p2": { "hand": [FLOOD, FILLER, "core-t-coin"], "field": [{ "def": VANILLA, "lane": 2 }], "library": LIBRARY },
            }));
            let prey = unit_at(&g, P2, 2, "setup: p2 should hold Mr. Vanilla in lane 2");
            g.play(KPOP, target(&prey.id));
            g.end_turn();

            g.play("core-t-coin", json!({}));
            g.play(FLOOD, json!({}));
            g.play(&prey, json!({ "zone": 2 }));
            assert_eq!(unit_id(&g, P2, 2), Some(prey.id.clone()));
            until_active(&mut g, P1);

            assert_eq!(unit_id(&g, P2, 2), Some(prey.id.clone()));
            assert_eq!(g.card(&prey).controller, P2);
            assert!(!g.card(&prey).radiant);
            assert!(!radiant_set_on(&g, &prey.id));
        }

        #[test]
        fn r282_a_target_dormant_under_a_stack_pile_is_neither_taken_nor_made_radiant_nor_is_the_card_on_top_r13() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "hand": [radiant_kpop(), FILLER], "field": [{ "def": VANILLA, "lane": 1 }], "library": LIBRARY },
                "p2": { "hand": [FIENDER, FILLER], "field": [{ "def": SEVEN_SEVEN, "lane": 2 }], "library": LIBRARY },
            }));
            let prey = unit_at(&g, P2, 2, "setup: p2 should hold the 7/7 in lane 2");
            g.play(KPOP, target(&prey.id));
            g.end_turn();

            // p2 stacks Felinor Fiender onto the prey's zone: the prey is dormant (§3.2, R13).
            g.play(FIENDER, json!({ "zone": 2 }));
            let fiender = match g.unit(P2, 2) {
                Some(unit) if unit.def_id == FIENDER => unit.clone(),
                _ => panic!("the Fiender should top lane 2"),
            };
            until_active(&mut g, P1);

            assert_eq!(g.card(&prey).controller, P2);
            assert!(!g.card(&prey).radiant);
            assert_eq!(g.card(&fiender).controller, P2);
            assert!(!g.card(&fiender).radiant);
            assert_eq!(count_events(&g, GameEventType::RadiantSet), 0);
        }

        #[test]
        fn r282_a_target_already_this_player_s_is_not_made_radiant_the_delayed_steal_took_nothing_r76() {
            crate::register_all();
            let mut g = scenario(json!({
                // K-Pop Fanatic costs 1 and Snom Bunny Mind Control 4.
                "p1": { "hand": [radiant_kpop(), MIND_CONTROL], "library": LIBRARY, "mana": 5 },
                "p2": { "hand": [FILLER], "field": [{ "def": SEVEN_SEVEN, "lane": 2 }], "library": LIBRARY },
            }));
            let prey = unit_at(&g, P2, 2, "setup: p2 should hold the 7/7 in lane 2");
            g.play(KPOP, target(&prey.id));
            // Base #49 takes the same permanent this turn, with no rider of its own.
            g.play(MIND_CONTROL, target(&prey.id));
            assert_eq!(g.card(&prey).controller, P1);

            until_active(&mut g, P2);
            until_active(&mut g, P1);

            assert_eq!(g.card(&prey).controller, P1);
            assert!(!g.card(&prey).radiant);
            assert!(!radiant_set_on(&g, &prey.id));
            assert_eq!(count_events(&g, GameEventType::ControlChanged), 1);
        }

        #[test]
        fn r282_a_full_row_keeps_the_target_with_the_opponent_and_it_is_not_made_radiant_r15() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": {
                    "hand": [radiant_kpop(), FILLER],
                    "field": [VANILLA, VANILLA, VANILLA, VANILLA],
                    "library": LIBRARY,
                },
                "p2": { "hand": [FILLER], "field": [{ "def": SEVEN_SEVEN, "lane": 2 }], "library": LIBRARY },
            }));
            let prey = unit_at(&g, P2, 2, "setup: p2 should hold the 7/7 in lane 2");
            // K-Pop Fanatic takes p1's last free zone, lane 5.
            g.play(KPOP, target(&prey.id));
            assert_eq!(g.unit(P1, 5).map(|unit| unit.def_id.clone()), Some(KPOP.to_string()));

            g.end_turn();
            g.end_turn();

            assert_eq!(g.state().active, P1);
            assert_eq!(unit_id(&g, P2, 2), Some(prey.id.clone()));
            assert_eq!(g.card(&prey).controller, P2);
            assert!(!g.card(&prey).radiant);
            assert!(!radiant_set_on(&g, &prey.id));
            assert!(g.state().delayed.is_empty());
        }
    }

    mod n50_k_pop_fanatic_r282_the_rider_is_the_face_its_cry_ran {
        use super::*;

        #[test]
        fn r282_a_base_k_pop_fanatic_made_radiant_after_its_cry_steals_without_the_rider() {
            crate::register_all();
            // The delayed steal carries the face the Cry resolved with (§10.6, R126): the base face's steal
            // has no rider, whatever the Fanatic is by the time it fires. (A Radiant one that has died since
            // still applies its rider: "radiant: Divine Shield eats the first hit …" above.)
            let mut g = scenario(json!({
                "p1": { "hand": [KPOP, { "def": "core-029", "radiant": true }, FILLER], "library": LIBRARY, "mana": 20 },
                "p2": { "hand": [FILLER], "field": [{ "def": SEVEN_SEVEN, "lane": 2 }], "library": LIBRARY },
            }));
            let prey = unit_at(&g, P2, 2, "setup: p2 should hold the 7/7 in lane 2");
            g.play(KPOP, target(&prey.id));
            // Radiant GIGA Glowy Jelly Bean makes every permanent p1 controls Radiant, the Fanatic included.
            g.play("core-029", json!({}));
            assert!(g.card(KPOP).radiant);

            until_active(&mut g, P2);
            until_active(&mut g, P1);

            assert_eq!(g.card(&prey).controller, P1);
            assert!(!g.card(&prey).radiant);
            assert!(!radiant_set_on(&g, &prey.id));
        }
    }

    mod n50_k_pop_fanatic_a_radiant_flip_on_the_field_adds_divine_shield_s5_2 {
        use super::*;

        #[test]
        fn s5_2_a_base_n50_whose_granted_divine_shield_was_spent_gets_its_radiant_face_s_divine_shield_when_n81_radiates_it() {
            crate::register_all();
            // #63 Plastic Surgery grants one random keyword (R21); pick the cursor whose roll is Divine
            // Shield, the only way a base #50 (no keywords) ever has one.
            let mut found: Option<Scenario> = None;
            let mut cursor: u32 = 0;
            while cursor < 200 && found.is_none() {
                let mut trial = scenario(json!({
                    "seed": "r5-ds-flip",
                    "p1": { "hand": [SURGERY, MENACE], "field": [KPOP, SAINTESS], "mana": 20 },
                    "p2": { "field": [MENACE] },
                }));
                trial.state_mut().rng_cursor = cursor;
                let kpop_id = trial.card(KPOP).id.clone();
                trial.play(SURGERY, target(&kpop_id));
                if trial.stats(KPOP).keywords.iter().any(|k| k.kind() == KeywordKind::DivineShield) {
                    found = Some(trial);
                }
                cursor += 1;
            }
            let mut s = found.expect("no cursor below 200 grants #50 Divine Shield");
            let kpop = s.card(KPOP).clone();
            let menace = s.unit(P2, 1).expect("p2's #19").clone();

            // The granted shield takes #19's strike back and is spent (§6.1).
            s.attack(&kpop, &menace);
            assert_eq!(s.card(&kpop).damage, 0);
            assert!(!s.stats(&kpop).keywords.iter().any(|k| k.kind() == KeywordKind::DivineShield));

            // #81 dies to #19 and its Death makes p1's other units Radiant: #50's radiant face prints
            // Divine Shield, a keyword its base face does not have, so it applies at once (§5.2) — the same
            // way a spent shield comes back when the keyword is granted again (§10.4, `grant_to`).
            s.attack(SAINTESS, &menace);
            assert!(s.card(&kpop).radiant);
            let kinds: Vec<KeywordKind> = s.stats(&kpop).keywords.iter().map(|k| k.kind()).collect();
            assert!(kinds.contains(&KeywordKind::DivineShield));

            // And it works as one: on p2's turn #19's 9 is negated whole, where the radiant 5/5 would die.
            s.end_turn();
            s.attack(&menace, &kpop);
            s.expect_in_zone(&kpop, "field");
            assert_eq!(s.card(&kpop).damage, 0);
        }
    }

    mod n50_k_pop_fanatic_r437_the_pending_steal_marks_its_target {
        use super::*;

        #[test]
        fn r437_the_cry_marks_its_target_purple_in_both_views_and_the_mark_goes_when_the_steal_resolves() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "hand": [KPOP, FILLER], "library": LIBRARY },
                "p2": { "hand": [FILLER], "field": [{ "def": SEVEN_SEVEN, "lane": 2 }], "library": LIBRARY },
            }));
            let prey = unit_at(&g, P2, 2, "setup: p2 should hold the 7/7 in lane 2");
            g.play(KPOP, target(&prey.id));

            assert_eq!(mark_events(g.last_events()), vec![marked(&prey.id, true)]);
            assert_eq!(marks_in(&g.view(P1), &prey.id), json!([steal_mark()]));
            assert_eq!(marks_in(&g.view(P2), &prey.id), json!([steal_mark()]));
            // K-Pop Fanatic itself carries none.
            let fanatic_id = unit_id(&g, P1, 1).unwrap_or_default();
            assert_eq!(marks_in(&g.view(P1), &fanatic_id), Value::Null);

            g.end_turn();
            // Still waiting through p2's turn.
            assert_eq!(marks_in(&g.view(P2), &prey.id), json!([steal_mark()]));
            g.end_turn();

            // Stolen at p1's start of turn: the mark is gone, from both views, with a `marked` removal.
            assert_eq!(g.card(&prey).controller, P1);
            assert_eq!(marks_in(&g.view(P1), &prey.id), Value::Null);
            assert_eq!(marks_in(&g.view(P2), &prey.id), Value::Null);
            let added: Vec<Value> = mark_events(g.events()).iter().map(|event| event["added"].clone()).collect();
            assert_eq!(added, vec![json!(true), json!(false)]);
            assert!(g.state().marks.is_none());
        }

        #[test]
        fn r437_r76_a_steal_that_fizzles_on_a_card_already_this_player_s_takes_the_mark_away_too() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "hand": [KPOP, MIND_CONTROL], "library": LIBRARY, "mana": 5 },
                "p2": { "hand": [FILLER], "field": [{ "def": SEVEN_SEVEN, "lane": 2 }], "library": LIBRARY },
            }));
            let prey = unit_at(&g, P2, 2, "setup: p2 should hold the 7/7 in lane 2");
            g.play(KPOP, target(&prey.id));
            g.play(MIND_CONTROL, target(&prey.id));
            // Taking control is not leaving the field (R171): the steal still waits, and so does its mark.
            assert_eq!(marks_in(&g.view(P1), &prey.id), json!([steal_mark()]));

            until_active(&mut g, P2);
            until_active(&mut g, P1);

            assert_eq!(marks_in(&g.view(P1), &prey.id), Value::Null);
            let added: Vec<Value> = mark_events(g.events()).iter().map(|event| event["added"].clone()).collect();
            assert_eq!(added, vec![json!(true), json!(false)]);
        }

        #[test]
        fn r437_r174_the_mark_goes_the_moment_the_target_leaves_the_field_with_the_steal() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "hand": [KPOP, FILLER], "library": LIBRARY },
                // Flood (4) bounces every unit: p2 takes the prey back to hand on its own turn.
                "p2": { "hand": [FLOOD, FILLER], "field": [{ "def": SEVEN_SEVEN, "lane": 2 }], "library": LIBRARY },
            }));
            let prey = unit_at(&g, P2, 2, "setup: p2 should hold the 7/7 in lane 2");
            g.play(KPOP, target(&prey.id));
            g.end_turn();

            g.play(FLOOD, json!({}));

            g.expect_in_zone(&prey, "hand");
            assert!(g.state().delayed.is_empty());
            assert_eq!(mark_events(g.last_events()), vec![marked(&prey.id, false)]);
            // The removal comes after the bounce that ended it.
            let types: Vec<GameEventType> = g.last_events().iter().map(|event| event.event_type()).collect();
            let last_marked = types
                .iter()
                .rposition(|kind| *kind == GameEventType::Marked)
                .map_or(-1, |index| index as i64);
            let first_bounced = types
                .iter()
                .position(|kind| *kind == GameEventType::Bounced)
                .map_or(-1, |index| index as i64);
            assert!(last_marked > first_bounced);
            assert!(g.state().marks.is_none());
        }

        #[test]
        fn r437_r33_a_face_down_target_p1_sees_the_mark_on_its_back_and_the_event_as_the_sentinel_p2_sees_both_in_full() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "hand": [KPOP, FILLER], "library": LIBRARY },
                "p2": { "hand": [FILLER], "backrow": [{ "def": "core-096", "lane": 3 }], "library": LIBRARY },
            }));
            let trap = g
                .backrow(P2, 3)
                .expect("setup: p2 should hold a face-down trap in backrow lane 3")
                .clone();
            g.play(KPOP, target(&trap.id));

            let p1_view = g.view(P1);
            let back = serde_json::to_value(&p1_view.opponent.backrow[2]).expect("a backrow view serialises");
            assert_eq!(back["faceDown"], json!(true));
            assert_eq!(back["marks"], json!([steal_mark()]));
            assert!(back.get("defId").is_none());
            let seen = mark_events(&p1_view.events);
            assert_eq!(seen, vec![marked("hidden", true)]);
            assert_eq!(marks_in(&g.view(P2), &trap.id), json!([steal_mark()]));
            let p2_marks = mark_events(&g.view(P2).events);
            assert_eq!(p2_marks.first().map(|event| event["instanceId"].clone()), Some(json!(trap.id)));
        }

        /// One action of the folded game: its nonce, `reduce`, and the log.
        fn act(log: &mut Vec<Action>, state: &GameState, body: Value) -> GameState {
            let mut action = body;
            action["nonce"] = json!(format!("kpop-mark-{}", log.len() + 1));
            let action: Action = json_as(action);
            let result = reduce(state, &action);
            if let Some(error) = result.error {
                panic!("{error}");
            }
            log.push(action);
            result.state
        }

        #[test]
        fn r437_s9_3_in_a_real_game_the_mark_rides_the_log_folding_it_rebuilds_the_same_marked_state_and_views() {
            crate::register_all();
            // p1's deck holds K-Pop Fanatic; p2's cheap Units give it a target on p1's second turn.
            let p1_deck: Vec<String> = [
                KPOP, "core-002", "core-005", "core-006", "core-008", "core-011", "core-012", "core-013", "core-015",
                "core-016", "core-019", "core-020", "core-025", "core-026", "core-032", "core-036", "core-043",
                "core-044", "core-053", "core-055",
            ]
            .iter()
            .map(|id| id.to_string())
            .collect();
            let p2_deck: Vec<String> = [
                "core-015", "core-011", "core-004", "core-002", "core-005", "core-006", "core-008", "core-012",
                "core-013", "core-016", "core-019", "core-020", "core-025", "core-026", "core-032", "core-036",
                "core-043", "core-044", "core-053", "core-055",
            ]
            .iter()
            .map(|id| id.to_string())
            .collect();
            let mut log: Vec<Action> = Vec::new();

            let mut found: Option<(String, GameState)> = None;
            let mut at = 0;
            while at < 300 && found.is_none() {
                let seed = format!("kpop-mark-{at}");
                let begun = begin_game(&create_game(&CreateGameOptions {
                    seed: seed.clone(),
                    decks: (p1_deck.clone(), p2_deck.clone()),
                    ..Default::default()
                }))
                .state;
                let p1_has = begun.players.p1.hand.iter().any(|card| card.def_id == KPOP);
                let p2_has = begun.players.p2.hand.iter().any(|card| card.def_id == "core-015");
                if p1_has && p2_has && begun.pending.is_none() {
                    found = Some((seed, begun));
                }
                at += 1;
            }
            let (seed, mut state) = found.expect("a seed below 300 deals both openings");

            for player in [P1, P2] {
                let keep: Vec<String> = state.players[player].hand.iter().map(|card| card.id.clone()).collect();
                state = act(&mut log, &state, json!({ "type": "mulligan", "keep": keep, "playerId": player }));
            }
            state = act(&mut log, &state, json!({ "type": "endTurn", "playerId": "p1" }));
            let token = state
                .players
                .p2
                .hand
                .iter()
                .find(|card| card.def_id == "core-015")
                .expect("p2 should hold Me and Mr Token")
                .clone();
            state = act(
                &mut log,
                &state,
                json!({ "type": "play", "instanceId": token.id, "zone": { "row": "units", "lane": 1 }, "playerId": "p2" }),
            );
            state = act(&mut log, &state, json!({ "type": "endTurn", "playerId": "p2" }));
            let fanatic = state
                .players
                .p1
                .hand
                .iter()
                .find(|card| card.def_id == KPOP)
                .expect("p1 should hold K-Pop Fanatic")
                .clone();
            state = act(
                &mut log,
                &state,
                json!({
                    "type": "play",
                    "instanceId": fanatic.id,
                    "targets": [{ "pick": "instance", "instanceId": token.id }],
                    "playerId": "p1",
                }),
            );

            assert_eq!(marks_in(&view_for(&state, P1), &token.id), json!([steal_mark()]));
            assert_eq!(marks_in(&view_for(&state, P2), &token.id), json!([steal_mark()]));

            let replayed = fold(&json_as::<FoldArgs>(json!({
                "seed": seed,
                "decks": [p1_deck, p2_deck],
                "log": log,
            })));
            assert!(replayed.errors.is_empty());
            assert_eq!(hash_state(&replayed.state), hash_state(&state));
            for viewer in [P1, P2] {
                assert_eq!(view_for(&replayed.state, viewer), view_for(&state, viewer));
            }
        }
    }
}
