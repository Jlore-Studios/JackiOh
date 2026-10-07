//! C #14 Shadowstep (SPEC §8.6 row 14, §4.5, §6.2 Replacement, §6.3 Flicker; R4, R11, R12, R33, R57,
//! R61, R64, R69, R78, R97, R99, R317). Trap, cost 2, Common.
//!   Base:    "Activates when any of your Units die: Return them to your hand. They cost ({setCost})."
//!   Radiant: "Activates when any of your Units would die: Flicker them instead, so they survive. Add a
//!            copy of each to your hand. The copies cost ({setCost})."
//!   Engine:  "Base: one firing covers every Unit of yours that one state-check pass collects (§4.5);
//!            each card still in a graveyard afterwards goes to its owner's hand (§3.2) with
//!            `costOverride` 0, the hand cap applying (R4). Tokens have ceased to exist (R11); a Reborn
//!            unit that came back is on the field, not in a graveyard, and is skipped. Radiant: a
//!            replacement (§6.2 Replacement) at the "would die" point, §4.5 step 1, before cards move:
//!            those units leave the collection and Flicker (§6.3): back in their zones, reset (R78), at
//!            full health, summoning sick, no Cry, no Death; a fresh copy of each (radiant flag kept,
//!            R57) goes to your hand with `costOverride` 0. Tunes: cost 0 ↓."
//!
//! THE BASE FACE answers a `destroyed` event of a Unit of yours (R99: the condition is its `when`, so
//! an enemy's death leaves it set). §4.5 step 1 collects a pass's deaths together and reports each with
//! its own `destroyed`, one after another, so the firing reads the whole run of `destroyed` events the
//! one it answers stands in — that pass — and takes every Unit of yours among them: one firing for all
//! of them. By the time a trap answers, the pass is over: a unit token has ceased to exist (R11) and a
//! Reborn unit is back on the field, so only the cards still in a graveyard go back, each to its
//! owner's hand (§3.2, R12) through §2.4's pipeline (a full hand burns it, R4, R317), costing the
//! card's number (`costOverride`, `param(ctx, "setCost")`, which R78 keeps in every zone).
//!
//! "Your Units" are the Units you controlled as they died — the event's `controller` (R172: a stolen
//! unit dies as its controller's) — while each card still goes back to its owner's hand (§3.2).
//!
//! THE RADIANT FACE is a replacement at "would die" (B5 E5, `Script.replacements`): §4.5 step 1 offers
//! it the units the check collected, and it takes its controller's among them — one firing for all —
//! and Flickers them in place (E22: reset, full health, summoning sick, no Cry, no Death, no Reborn).
//! A face-down Trap that replaces fires (`trapFired`, face-up) and is spent. Its follow-up, owed after
//! the event (B5 E5), adds a fresh copy of each flickered card to your hand (its radiant flag kept, R57)
//! costing the card's number; a copy in your hand is yours alone to read (R97).

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-014";

/// The `destroyed` events of the state-check pass the answered one belongs to: the unbroken run of
/// `destroyed` reports it stands in (a card a replacement took elsewhere reports its landing there
/// instead, and still belongs to the run). The answered event itself when the list holds no run.
///
/// (TS `passOf(ctx, answered: Destroyed)`: `answered_id` is the answered event's `instanceId`.)
fn pass_of(ctx: &EffectContext<'_>, answered: &GameEvent, answered_id: &str) -> Vec<GameEvent> {
    let events: &[GameEvent] = &ctx.events[..];
    let found = events.iter().rposition(
        |event| matches!(event, GameEvent::Destroyed { instance_id, .. } if instance_id == answered_id),
    );
    let Some(at) = found else {
        return vec![answered.clone()];
    };
    let in_pass =
        |event: Option<&GameEvent>| matches!(event, Some(GameEvent::Destroyed { .. } | GameEvent::Exiled { .. }));
    let mut start = at;
    while start > 0 && in_pass(events.get(start - 1)) {
        start -= 1;
    }
    let mut end = at;
    while in_pass(events.get(end + 1)) {
        end += 1;
    }
    events[start..=end]
        .iter()
        .filter(|event| matches!(event, GameEvent::Destroyed { .. }))
        .cloned()
        .collect()
}

fn shadowstep() -> TriggerDef {
    TriggerDef::new("shadowstep", &[GameEventType::Destroyed], |ctx, event| {
        let GameEvent::Destroyed { instance_id: answered_id, .. } = event else {
            return vec![];
        };
        let cost = param(&*ctx, "setCost");
        pass_of(ctx, event, answered_id)
            .into_iter()
            .filter_map(|died| match died {
                GameEvent::Destroyed { instance_id, controller, .. } => Some((instance_id, controller)),
                _ => None,
            })
            .filter(|(_, controller)| *controller == ctx.controller)
            .filter(|(instance_id, _)| {
                matches!(find_instance(&ctx.state, instance_id).map(|card| &card.zone), Some(Zone::Graveyard { .. }))
            })
            .map(|(instance_id, _)| {
                add_to_hand(json_as(json!({
                    "instance": { "of": "instance", "instanceId": instance_id },
                    "costOverride": cost,
                })))
            })
            .collect()
    })
    .with_when(|ctx, event| matches!(event, GameEvent::Destroyed { controller, .. } if *controller == ctx.controller))
}

pub fn script() -> CardScripts {
    let base = Script { triggers: vec![shadowstep()], ..Script::default() };

    let radiant = Script {
        replacements: vec![ReplacementDef {
            id: "shadowstep".into(),
            on: ReplacementMoment::WouldDie,
            where_: None,
            when: None,
            instead: ReplacementInstead { flicker: Some(InsteadFlicker::Yours), ..ReplacementInstead::default() },
            then: Some("copies".into()),
            by: None,
        }],
        resume: IndexMap::from([(
            "copies",
            hook(|ctx| {
                let cost = param(&*ctx, "setCost");
                replacement_of(&*ctx)
                    .map(|record| record.flickered.clone().unwrap_or_default())
                    .unwrap_or_default()
                    .into_iter()
                    .map(|card| {
                        add_to_hand(json_as(json!({
                            "defId": card.def_id,
                            "radiant": card.radiant,
                            "costOverride": cost,
                        })))
                    })
                    .collect()
            }),
        )]),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// C #14 Shadowstep — SPEC §8.6 row 14, BUILD M9 Classic row C 14: "Face-down (R33); fires once for
// all of your Units one state-check pass collects; each card still in a graveyard afterwards returns
// to its owner's hand (a stolen one to the opponent's, §3.2) and costs (0) (`costOverride`, kept,
// R78), a full hand burning the overflow (R317); unit tokens have ceased to exist (R11), a Reborn Unit
// already back is skipped, and an Indestructible Unit is collected only when its max health falls to
// 0 (R69); enemy deaths don't fire it; radiant: fires in the "would die" window at §4.5 step 1: those
// Units leave the collection and flicker (the same zone, reset per R78, full health, summoning sick,
// no Cry, no Death), and a fresh copy of each (its Radiant flag kept, R57) goes to your hand and costs
// (0); the copies are never named in the opponent's view (R97); its tuned number (cost) reads through
// `param()` (R386)".
//
// The stolen-unit case needs the `destroyed` event to say who controlled the unit as it died (it names
// the owner only): it waits for that engine change.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SHADOWSTEP: &str = "classic-014";
    const BIG_FELINOR: &str = "core-043"; // (4) Unit 3/10: "Cry: Destroy all non-Felinor Units."
    const MIND_CONTROL: &str = "core-049"; // (4) Spell: "Steal target enemy permanent."
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const MENACE: &str = "core-019"; // (3) Unit 9/9
    const TIMMY: &str = "core-011"; // (1) Unit 3/3
    const DEFENDER: &str = "core-003"; // (1) Unit 1/1 Taunt, Divine Shield, Reborn
    const STATE_OF_GAME: &str = "classic-041"; // (1) Unit 3/3 Indestructible
    const SAINTESS: &str = "core-081"; // (1) Unit 2/2: "Death: Make your other Units Radiant."
    const MR_TOKEN: &str = "core-015"; // (1) Unit 1/1: "Cry: Summon a Rush Token."
    const RUSH_TOKEN: &str = "core-t-rush"; // (1) Unit token 3/3
    const FELINORS: &str = "core-012"; // (2) Unit 3/4, a Felinor Big Felinor spares
    const STOCKPILE: &str = "core-005"; // (1) Spell, a spare card (§2.5)
    const SUPPRESSIVE_AURA: &str = "core-046"; // (2) Field Spell: "Aura: All Units have −1/−1."

    /// p1 sets the trap; p2, active, plays Big Felinor and destroys every non-Felinor Unit in one pass.
    /// (TS `wipe(radiantFace, mine, opts = {})`, `opts`' keys `myHand` and `theirs` as TS named them.)
    fn wipe(radiant_face: bool, mine: Value, opts: Value) -> Scenario {
        scenario(json!({
            "active": "p2",
            "p1": {
                "hand": opts.get("myHand").cloned().unwrap_or(json!([STOCKPILE])),
                "field": mine,
                "backrow": [{ "def": SHADOWSTEP, "faceUp": false, "radiant": radiant_face }],
                "library": [STOCKPILE],
            },
            "p2": {
                "hand": [BIG_FELINOR, STOCKPILE],
                "field": opts.get("theirs").cloned().unwrap_or(json!([])),
                "library": [STOCKPILE],
            },
        }))
    }

    fn fired(s: &Scenario) -> usize {
        s.events().iter().filter(|event| event.event_type() == GameEventType::TrapFired).count()
    }

    fn mine_on_field(s: &Scenario) -> Vec<Option<String>> {
        (1..=5).map(|lane| s.unit(P1, lane).map(|card| card.def_id)).collect()
    }

    fn hand_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.hand(player).into_iter().map(|card| card.def_id).collect()
    }

    use crate::js;

    fn backrow_def(s: &Scenario, player: PlayerId, lane: i32) -> Option<String> {
        s.backrow(player, lane).map(|card| card.def_id)
    }

    fn burned_ids(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Burned { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn declares_its_one_number_the_cost_the_cards_come_back_at_r386_0_never_below_0() {
        assert_eq!(
            js(&crate::card_def(SHADOWSTEP).params),
            json!([{ "key": "setCost", "base": 0, "radiant": 0, "better": "down", "step": 1, "min": 0 }])
        );
        let CardScripts { base, radiant } = script();
        let ons: Vec<Vec<GameEventType>> = base.triggers.iter().map(|trigger| trigger.on.clone()).collect();
        assert_eq!(ons, vec![vec![GameEventType::Destroyed]]);
        let moments: Vec<ReplacementMoment> = radiant.replacements.iter().map(|entry| entry.on).collect();
        assert_eq!(moments, vec![ReplacementMoment::WouldDie]);
    }

    mod base {
        use super::*;

        #[test]
        fn r33_it_sits_face_down() {
            let s = wipe(false, json!([VANILLA]), json!({}));
            assert_eq!(js(&s.view(P2))["opponent"]["backrow"][0], json!({ "faceDown": true, "cost": 2 }));
        }

        #[test]
        fn fires_once_for_all_of_your_units_one_state_check_pass_collects_and_returns_each_to_your_hand() {
            let mut s = wipe(false, json!([VANILLA, MENACE, TIMMY]), json!({}));
            let cards = [s.card(VANILLA).clone(), s.card(MENACE).clone(), s.card(TIMMY).clone()];
            s.play(BIG_FELINOR, json!({}));
            assert_eq!(fired(&s), 1);
            for card in &cards {
                s.expect_in_zone(card, "hand");
            }
            assert_eq!(hand_defs(&s, P1), [STOCKPILE, VANILLA, MENACE, TIMMY]);
            s.expect_in_zone(SHADOWSTEP, "graveyard");
        }

        #[test]
        fn r78_they_cost_0_costoverride_kept_in_every_zone() {
            let mut s = wipe(false, json!([MENACE]), json!({}));
            let menace = s.card(MENACE).clone();
            s.play(BIG_FELINOR, json!({}));
            assert_eq!(s.card(&menace).cost_override, Some(0));
            assert_eq!(effective_cost(s.state(), s.card(&menace), Default::default()), 0);
        }

        #[test]
        fn enemy_deaths_don_t_fire_it() {
            let mut s = wipe(false, json!([FELINORS]), json!({ "theirs": [VANILLA, MENACE] }));
            s.play(BIG_FELINOR, json!({}));
            assert_eq!(fired(&s), 0);
            assert_eq!(backrow_def(&s, P1, 1), Some(SHADOWSTEP.to_string()));
            s.expect_in_zone(FELINORS, "field");
        }

        #[test]
        fn the_enemy_s_units_that_die_in_the_same_pass_stay_in_their_graveyard() {
            let mut s = wipe(false, json!([VANILLA]), json!({ "theirs": [MENACE] }));
            let theirs = s.card(MENACE).clone();
            s.play(BIG_FELINOR, json!({}));
            s.expect_in_zone(&theirs, "graveyard");
            s.expect_in_zone(VANILLA, "hand");
        }

        #[test]
        fn r11_a_unit_token_has_ceased_to_exist_nothing_of_it_comes_back() {
            let mut s = wipe(false, json!([RUSH_TOKEN, VANILLA]), json!({}));
            let token = s.card(RUSH_TOKEN).clone();
            s.play(BIG_FELINOR, json!({}));
            s.expect_in_zone(&token, "gone");
            assert_eq!(hand_defs(&s, P1), [STOCKPILE, VANILLA]);
        }

        #[test]
        fn a_reborn_unit_already_back_on_the_field_is_skipped() {
            let mut s = wipe(false, json!([DEFENDER, VANILLA]), json!({}));
            let defender = s.card(DEFENDER).clone();
            s.play(BIG_FELINOR, json!({}));
            s.expect_in_zone(&defender, "field");
            assert_eq!(hand_defs(&s, P1), [STOCKPILE, VANILLA]);
        }

        #[test]
        fn r69_an_indestructible_unit_a_destroy_leaves_standing_is_not_collected_and_fires_nothing() {
            let mut s = wipe(false, json!([STATE_OF_GAME]), json!({}));
            s.play(BIG_FELINOR, json!({}));
            s.expect_in_zone(STATE_OF_GAME, "field");
            assert_eq!(fired(&s), 0);
        }

        #[test]
        fn r69_an_indestructible_unit_whose_max_health_falls_to_0_is_collected_fires_it_and_comes_back() {
            // A State of the Game standing as a 1/1 meets Suppressive Aura's −1/−1: max health 0.
            let mut s = scenario(json!({
                "active": "p2",
                "p1": {
                    "hand": [STOCKPILE],
                    "field": [{ "def": STATE_OF_GAME, "statsOverride": { "attack": 1, "health": 1 } }, VANILLA],
                    "backrow": [{ "def": SHADOWSTEP, "faceUp": false }],
                },
                "p2": { "hand": [SUPPRESSIVE_AURA, STOCKPILE] },
            }));
            let state = s.card(STATE_OF_GAME).clone();
            s.play(SUPPRESSIVE_AURA, json!({ "zone": 1 }));
            assert_eq!(fired(&s), 1);
            s.expect_in_zone(&state, "hand");
            assert_eq!(s.card(&state).cost_override, Some(0));
            s.expect_in_zone(VANILLA, "field");
        }

        #[test]
        fn your_unit_the_opponent_controls_is_theirs_while_it_is_on_the_field_its_death_leaves_it_set() {
            // "Your Units" on the field are the ones you control (§3.2, R12): a Menace of yours they stole
            // dies under their control, so it is not one of your Units dying.
            let mut s = scenario(json!({
                "active": "p2",
                "p1": {
                    "hand": [BIG_FELINOR, STOCKPILE],
                    "field": [MENACE],
                    "backrow": [{ "def": SHADOWSTEP, "faceUp": false }],
                    "library": [STOCKPILE],
                },
                "p2": { "hand": [MIND_CONTROL, STOCKPILE], "library": [STOCKPILE, STOCKPILE] },
            }));
            let menace = s.card(MENACE).clone();
            s.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": menace.id }] }));
            assert_eq!(s.card(&menace).controller, P2);
            s.end_turn();
            s.play(BIG_FELINOR, json!({}));
            assert_eq!(fired(&s), 0);
            s.expect_in_zone(&menace, "graveyard");
            assert_eq!(backrow_def(&s, P1, 1), Some(SHADOWSTEP.to_string()));
        }

        #[test]
        fn r317_a_full_hand_burns_the_overflow_into_your_graveyard() {
            let nine = vec![STOCKPILE; 9];
            let mut s = wipe(false, json!([VANILLA, MENACE]), json!({ "myHand": nine }));
            let vanilla = s.card(VANILLA).clone();
            let menace = s.card(MENACE).clone();
            s.play(BIG_FELINOR, json!({}));
            s.expect_in_zone(&vanilla, "hand");
            s.expect_in_zone(&menace, "graveyard");
            assert_eq!(burned_ids(&s), vec![menace.id.clone()]);
        }

        #[test]
        fn its_death_hooks_still_happen_the_units_died() {
            let mut s = wipe(false, json!([SAINTESS, VANILLA]), json!({}));
            s.play(BIG_FELINOR, json!({}));
            s.expect_in_zone(SAINTESS, "hand");
            s.expect_events(json!(["destroyed", "trapFired"]));
        }

        #[test]
        fn r97_once_in_your_hand_the_returned_cards_are_named_in_none_of_the_opponent_s_view() {
            let mut s = wipe(false, json!([VANILLA, MENACE]), json!({}));
            let cards: Vec<CardInstance> = vec![s.card(VANILLA).clone(), s.card(MENACE).clone()];
            s.play(BIG_FELINOR, json!({}));
            let theirs = serde_json::to_string(&s.view(P2)).unwrap();
            for card in &cards {
                assert!(!theirs.contains(&format!("\"{}\"", card.id)));
            }
            assert_eq!(js(&s.view(P2))["opponent"]["hand"], json!({ "count": 3 }));
        }

        #[test]
        fn sec3_2_a_stolen_unit_of_theirs_you_control_that_dies_returns_to_its_owner_s_hand() {
            let mut s = scenario(json!({
                "p1": { "hand": [MIND_CONTROL, STOCKPILE], "backrow": [{ "def": SHADOWSTEP, "faceUp": false }], "library": [STOCKPILE] },
                "p2": { "hand": [BIG_FELINOR, STOCKPILE], "field": [MENACE], "library": [STOCKPILE, STOCKPILE] },
            }));
            let menace = s.card(MENACE).clone();
            s.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": menace.id }] }));
            assert_eq!(s.card(&menace).controller, P1);
            s.end_turn();
            s.play(BIG_FELINOR, json!({}));
            assert_eq!(fired(&s), 1);
            s.expect_in_zone(&menace, "hand");
            assert!(s.hand(P2).iter().any(|card| card.id == menace.id));
        }

        #[test]
        fn r386_a_degrade_of_the_cost_makes_them_cost_1() {
            let mut s = wipe(false, json!([MENACE]), json!({}));
            step_param(s.card_mut(SHADOWSTEP), "setCost", 1);
            let menace = s.card(MENACE).clone();
            s.play(BIG_FELINOR, json!({}));
            assert_eq!(s.card(&menace).cost_override, Some(1));
        }
    }

    mod radiant {
        use super::*;

        fn copies(s: &Scenario) -> Vec<CardInstance> {
            s.hand(P1).into_iter().filter(|card| card.def_id != STOCKPILE).collect()
        }

        #[test]
        fn e5_fires_in_the_would_die_window_your_units_flicker_instead_and_stay_in_their_zones() {
            let mut s = wipe(true, json!([VANILLA, MENACE]), json!({}));
            let vanilla = s.card(VANILLA).clone();
            let menace = s.card(MENACE).clone();
            s.play(BIG_FELINOR, json!({}));
            assert_eq!(fired(&s), 1);
            assert_eq!(s.unit(P1, 1).map(|card| card.id), Some(vanilla.id.clone()));
            assert_eq!(s.unit(P1, 2).map(|card| card.id), Some(menace.id.clone()));
            assert!(!s
                .events()
                .iter()
                .any(|event| matches!(event, GameEvent::Destroyed { owner: PlayerId::P1, .. })));
            s.expect_events(json!(["trapFired", "flickered"]));
            s.expect_in_zone(SHADOWSTEP, "graveyard");
        }

        #[test]
        fn r78_flickered_reset_at_full_health_summoning_sick() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": {
                    "hand": [STOCKPILE],
                    "field": [{ "def": MENACE, "damage": 5 }],
                    "backrow": [{ "def": SHADOWSTEP, "faceUp": false, "radiant": true }],
                },
                "p2": { "hand": [BIG_FELINOR, STOCKPILE] },
            }));
            let menace = s.card(MENACE).clone();
            s.play(BIG_FELINOR, json!({}));
            s.expect_stats(&menace, json!({ "health": 9, "maxHealth": 9 }));
            assert_eq!(s.card(&menace).summoned_turn, Some(s.state().turn));
        }

        #[test]
        fn no_cry_and_no_death_a_flickered_saintess_makes_nothing_radiant_a_mr_token_summons_nothing() {
            let mut s = wipe(true, json!([SAINTESS, MR_TOKEN, VANILLA]), json!({}));
            s.play(BIG_FELINOR, json!({}));
            let expected: Vec<Option<String>> =
                [Some(SAINTESS), Some(MR_TOKEN), Some(VANILLA), None, None].iter().map(|lane| lane.map(str::to_string)).collect();
            assert_eq!(mine_on_field(&s), expected);
            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::RadiantSet));
            assert!(!s
                .events()
                .iter()
                .any(|event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == RUSH_TOKEN)));
        }

        #[test]
        fn r57_a_fresh_copy_of_each_goes_to_your_hand_its_radiant_flag_kept_costing_0() {
            let mut s = wipe(true, json!([{ "def": VANILLA, "radiant": true }, MENACE]), json!({}));
            let vanilla = s.card(VANILLA).clone();
            s.play(BIG_FELINOR, json!({}));
            let copies = copies(&s);
            let seen: Vec<(String, bool, Option<i32>)> =
                copies.iter().map(|card| (card.def_id.clone(), card.radiant, card.cost_override)).collect();
            assert_eq!(seen, vec![(VANILLA.to_string(), true, Some(0)), (MENACE.to_string(), false, Some(0))]);
            assert!(!copies.iter().any(|card| card.id == vanilla.id));
        }

        #[test]
        fn r97_the_copies_are_never_named_in_the_opponent_s_view() {
            let mut s = wipe(true, json!([VANILLA, MENACE]), json!({}));
            s.play(BIG_FELINOR, json!({}));
            let copies = copies(&s);
            let theirs = serde_json::to_string(&s.view(P2)).unwrap();
            for card in &copies {
                assert!(!theirs.contains(&format!("\"{}\"", card.id)));
            }
        }

        #[test]
        fn enemy_units_still_die_and_an_all_enemy_pass_leaves_it_set() {
            let mut s = wipe(true, json!([FELINORS]), json!({ "theirs": [VANILLA] }));
            s.play(BIG_FELINOR, json!({}));
            s.expect_in_zone(VANILLA, "graveyard");
            assert_eq!(fired(&s), 0);
            assert_eq!(backrow_def(&s, P1, 1), Some(SHADOWSTEP.to_string()));
        }

        #[test]
        fn r317_a_full_hand_burns_the_overflowing_copies() {
            let nine = vec![STOCKPILE; 9];
            let mut s = wipe(true, json!([VANILLA, MENACE]), json!({ "myHand": nine }));
            s.play(BIG_FELINOR, json!({}));
            assert_eq!(s.hand(P1).len(), 10);
            assert_eq!(burned_ids(&s).len(), 1);
        }

        #[test]
        fn r386_a_degrade_of_the_cost_makes_the_copies_cost_1() {
            let mut s = wipe(true, json!([MENACE]), json!({}));
            step_param(s.card_mut(SHADOWSTEP), "setCost", 1);
            s.play(BIG_FELINOR, json!({}));
            let copy = s.hand(P1).into_iter().find(|card| card.def_id == MENACE);
            assert_eq!(copy.and_then(|card| card.cost_override), Some(1));
        }
    }
}
