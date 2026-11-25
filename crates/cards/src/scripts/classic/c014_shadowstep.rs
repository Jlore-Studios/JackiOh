//! C #14 Shadowstep (SPEC §8.6 row 14, §4.5, §6.2 Replacement, §6.3 Flicker; R4, R11, R12, R33, R57,
//! R61, R64, R69, R78, R97, R99, R317, R4395000, R4395001, R4423000). Trap, cost 1 (2 until issue #3136), Common.
//!   Base:    "Activates when any of your Units die: Return them to your hand. They cost ({setCost})."
//!   Radiant: "Activates when any of your Units would die: Flicker them instead, so they survive. Add a
//!            copy of each to your hand. The copies cost ({setCost})."
//!
//! BASE: a `destroyed` of a Unit of yours, read by what the card was on the field as it died (R4423000,
//! `is_unit_death`; R4395000: a Field Spell, Trap or Field Trap of yours is none, alone or beside Units, unless
//! it died standing as a Unit, Animated, R4395001; a Unit standing in the backrow is one, R1081000; a Unit set
//! face-down by its Cloaked and destroyed before it reveals died as a Field Trap, none; `script_v0_6_9i` keeps
//! v0.6.9h's to v0.6.9i's read, the definition's type unless the action's own `destroyed` says Animated,
//! R767000). R99: the condition is its `when`, so an enemy's death leaves it
//! set. One firing reads the whole §4.5 step 1 pass it stands in and takes every Unit of yours (the
//! event's `controller`, R172). A token is gone by then (R11), a Reborn unit is back; the cards still in
//! a graveyard go to their owner's hand (§3.2, R12, §2.4; a full hand burns, R4, R317) at `setCost` (R78),
//! which is yours: a Unit you stole died into your graveyard as yours (R2618000).
//! RADIANT: a replacement at "would die" (B5 E5): your collected units Flicker (E22: reset, full health,
//! no Cry, no Death), then a definition-remade copy of each joins your hand with what R57 gives a copy
//! in a hand (its radiant flag, `statsOverride` and `tuning`, and R175's `armorOverride` with the size:
//! a token summoned at a size is still that size, #1803), but not its Chinese flag (R1300), unread by
//! them (R97).

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-014";

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

/// R4423000: a `destroyed` event whose card died as a Unit (`is_unit_death`). `by_print` is the read of
/// v0.6.9l and before (R767000): R4395000, by its definition's type on the face it died on, or, R4395001, an
/// Animated Field Spell or Trap that died standing in a unit zone. The trigger loop holds the event without
/// its `animated`, so that is read off the action's own `destroyed` of the same card.
fn is_unit(ctx: &EffectContext<'_>, died: &GameEvent, by_print: bool) -> bool {
    if !by_print {
        return is_unit_death(ctx.state, died);
    }
    let GameEvent::Destroyed { instance_id, def_id, radiant, .. } = died else {
        return false;
    };
    let animated = ctx.events.iter().any(|event| {
        matches!(event, GameEvent::Destroyed { instance_id: id, animated: Some(true), .. } if id == instance_id)
    });
    animated || card_type_of_face(ctx.state, def_id, *radiant == Some(true)) == CardType::Unit
}

/// `units_only` is R4395000's trigger; off, it is what v0.6.9g and before ran: any card of yours destroyed.
/// `by_print`: as `is_unit`'s.
fn shadowstep(units_only: bool, by_print: bool) -> TriggerDef {
    TriggerDef::new("shadowstep", &[GameEventType::Destroyed], move |ctx, event| {
        let GameEvent::Destroyed { instance_id: answered_id, .. } = event else {
            return vec![];
        };
        let cost = param(&*ctx, "setCost");
        pass_of(ctx, event, answered_id)
            .into_iter()
            .filter(|died| !units_only || is_unit(ctx, died, by_print))
            .filter_map(|died| match died {
                GameEvent::Destroyed { instance_id, controller, .. } => Some((instance_id, controller)),
                _ => None,
            })
            .filter(|(_, controller)| *controller == ctx.controller)
            .filter(|(instance_id, _)| {
                matches!(find_instance(ctx.state, instance_id).map(|card| &card.zone), Some(Zone::Graveyard { .. }))
            })
            .map(|(instance_id, _)| {
                add_to_hand(json_as(json!({
                    "instance": { "of": "instance", "instanceId": instance_id },
                    "costOverride": cost,
                })))
            })
            .collect()
    })
    .with_when(move |ctx, event| {
        matches!(event, GameEvent::Destroyed { controller, .. } if *controller == ctx.controller)
            && (!units_only || is_unit(ctx, event, by_print))
    })
}

fn scripts(copy_face: bool, units_only: bool, by_print: bool) -> CardScripts {
    let base = Script { triggers: vec![shadowstep(units_only, by_print)], ..Script::default() };

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
            hook(move |ctx| {
                let cost = param(&*ctx, "setCost");
                replacement_of(&*ctx)
                    .map(|record| record.flickered.clone().unwrap_or_default())
                    .unwrap_or_default()
                    .into_iter()
                    .map(|card| {
                        let mut args = json!({
                            "defId": card.def_id,
                            "radiant": card.radiant,
                            "costOverride": cost,
                        });
                        if copy_face
                            && let Some(source) = find_instance(ctx.state, &card.instance_id)
                        {
                            if let Some(stats) = source.stats_override {
                                args["statsOverride"] = json!(stats);
                            }
                            if let Some(armor) = source.armor_override {
                                args["armorOverride"] = json!(armor);
                            }
                            if let Some(tuning) = &source.tuning {
                                args["tuning"] = json!(tuning);
                            }
                        }
                        add_to_hand(json_as(args))
                    })
                    .collect()
            }),
        )]),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

pub fn script() -> CardScripts {
    scripts(true, true, false)
}

/// R767000: v0.4.12b and every version before it made Shadowstep's hand card from only its definition
/// and Radiant flag; `KEPT_SCRIPTS` folds those replays under that script.
pub fn script_v0_4_12b() -> CardScripts {
    scripts(false, false, true)
}

/// R4395000: v0.6.9g and every version before it answered every card of yours that was destroyed;
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_9g() -> CardScripts {
    scripts(true, false, true)
}

/// R4423000: v0.6.9i and the versions before it back to v0.6.9h read a death by the definition's type unless
/// the action's own `destroyed` said Animated, so a Unit set face-down by its Cloaked and destroyed before it
/// reveals answered it; `KEPT_SCRIPTS` folds those replays under it (R767000).
pub fn script_v0_6_9i() -> CardScripts {
    scripts(true, true, true)
}

// C #14 Shadowstep — SPEC §8.6 row 14, BUILD M9 Classic row C 14 (R33, §4.5, §3.2, R78, R317, R11, R69,
// R57, R97, R386).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::def_ids;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SHADOWSTEP: &str = "classic-014";
    const BIG_FELINOR: &str = "core-043"; // Big Felinor: "Cry: Destroy all non-Felinor Units."
    const MIND_CONTROL: &str = "core-049"; // Snom Bunny Mind Control: "Float reduces cost. Steal an enemy card."
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const MENACE: &str = "core-019"; // Midrange Menace
    const TIMMY: &str = "stand-in (1) 3/3 Rush, First Strike R6/6 Charge, First Strike";
    const DEFENDER: &str = "core-003"; // Right-house defender: Unit Taunt, Divine Shield, Reborn
    const STATE_OF_GAME: &str = "classic-041"; // State of the Game: Unit Indestructible
    const SAINTESS: &str = "core-081"; // Radiant Saintess: "Death: Make your other Units Radiant."
    const MR_TOKEN: &str = "core-015"; // Me and Mr Token: "Cry: Summon a Rush Token."
    const RUSH_TOKEN: &str = "core-t-rush"; // Rush Token: Unit token
    const FELINORS: &str = "core-012"; // Duplicating Felinors: a Felinor Big Felinor spares
    const STOCKPILE: &str = "core-005"; // Stockpile: a Spell, a spare card (§2.5)
    const SUPPRESSIVE_AURA: &str = "core-046"; // Suppressive Aura: "Aura: All Units get a stat penalty."
    const WALLS: &str = "classicplus-024"; // Crushing Walls: destroy every card in two lanes
    const FIRE: &str = "classicplus-t-ai-06"; // Datacenter Fire: destroy all Field Spells
    const FIELD_SPELL: &str = "core-064"; // Gifted Program: a Field Spell
    const TRAP: &str = "core-060"; // Bear Honeypot: a Trap
    const TESLA: &str = "classic-005"; // Tesla: a Field Trap, Animated

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

    /// p1 holds a Field Spell and a face-down base Shadowstep; p2 holds Datacenter Fire.
    fn datacenter() -> Scenario {
        scenario(json!({
            "active": "p2",
            "p1": {
                "hand": [STOCKPILE],
                "library": [STOCKPILE],
                "backrow": [
                    { "def": FIELD_SPELL, "lane": 1 },
                    { "def": SHADOWSTEP, "faceUp": false, "lane": 2 },
                ],
            },
            "p2": { "hand": [FIRE, STOCKPILE], "library": [STOCKPILE] },
        }))
    }

    /// p1 holds `field` and `backrow` (a face-down base Shadowstep among them); p2 holds Crushing Walls.
    fn walled(field: Value, backrow: Value) -> Scenario {
        scenario(json!({
            "active": "p2",
            "p1": { "hand": [STOCKPILE], "library": [STOCKPILE], "field": field, "backrow": backrow },
            "p2": { "hand": [WALLS, STOCKPILE], "library": [STOCKPILE] },
        }))
    }

    fn mine_on_field(s: &Scenario) -> Vec<Option<String>> {
        (1..=5).map(|lane| s.unit(P1, lane).map(|card| card.def_id)).collect()
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

    /// Issue #3136: it costs (1), from (2), on both faces.
    #[test]
    fn issue_3136_costs_1_refused_with_0_mana_on_both_faces() {
        for radiant in [false, true] {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": SHADOWSTEP, "radiant": radiant }, STOCKPILE] },
                "p2": { "hand": [STOCKPILE] },
            }));
            crate::expect_costs(&mut s, SHADOWSTEP, 1, json!({}));
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r33_it_sits_face_down() {
            let s = wipe(false, json!([VANILLA]), json!({}));
            assert_eq!(js(&s.view(P2))["opponent"]["backrow"][0], json!({ "faceDown": true, "cost": 1 }));
        }

        #[test]
        fn fires_once_for_all_of_your_units_one_state_check_pass_collects_and_returns_each_to_your_hand() {
            let mut s = wipe(false, json!([VANILLA, MENACE, TIMMY]), json!({}));
            let cards = [s.card(VANILLA).clone(), s.card(MENACE).clone(), s.card(TIMMY).clone()];
            s.play(BIG_FELINOR, json!({}));
            assert_eq!(s.count(GameEventType::TrapFired), 1);
            for card in &cards {
                s.expect_in_zone(card, "hand");
            }
            assert_eq!(def_ids(&s.hand(P1)), [STOCKPILE, VANILLA, MENACE, TIMMY]);
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
            assert_eq!(s.count(GameEventType::TrapFired), 0);
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
            assert_eq!(def_ids(&s.hand(P1)), [STOCKPILE, VANILLA]);
        }

        #[test]
        fn a_reborn_unit_already_back_on_the_field_is_skipped() {
            let mut s = wipe(false, json!([DEFENDER, VANILLA]), json!({}));
            let defender = s.card(DEFENDER).clone();
            s.play(BIG_FELINOR, json!({}));
            s.expect_in_zone(&defender, "field");
            assert_eq!(def_ids(&s.hand(P1)), [STOCKPILE, VANILLA]);
        }

        #[test]
        fn r69_an_indestructible_unit_a_destroy_leaves_standing_is_not_collected_and_fires_nothing() {
            let mut s = wipe(false, json!([STATE_OF_GAME]), json!({}));
            s.play(BIG_FELINOR, json!({}));
            s.expect_in_zone(STATE_OF_GAME, "field");
            assert_eq!(s.count(GameEventType::TrapFired), 0);
        }

        #[test]
        fn r69_an_indestructible_unit_whose_max_health_falls_to_0_is_collected_fires_it_and_comes_back() {
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
            assert_eq!(s.count(GameEventType::TrapFired), 1);
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
                "p2": { "hand": [MIND_CONTROL, STOCKPILE], "mana": 6, "library": [STOCKPILE, STOCKPILE] },
            }));
            let menace = s.card(MENACE).clone();
            s.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": menace.id }] }));
            assert_eq!(s.card(&menace).controller, P2);
            s.end_turn();
            s.play(BIG_FELINOR, json!({}));
            assert_eq!(s.count(GameEventType::TrapFired), 0);
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
        fn r2618000_a_stolen_unit_of_theirs_you_control_that_dies_returns_to_your_hand() {
            let mut s = scenario(json!({
                "p1": { "hand": [MIND_CONTROL, STOCKPILE], "mana": 6, "backrow": [{ "def": SHADOWSTEP, "faceUp": false }], "library": [STOCKPILE] },
                "p2": { "hand": [BIG_FELINOR, STOCKPILE], "field": [MENACE], "library": [STOCKPILE, STOCKPILE] },
            }));
            let menace = s.card(MENACE).clone();
            s.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": menace.id }] }));
            assert_eq!(s.card(&menace).controller, P1);
            s.end_turn();
            s.play(BIG_FELINOR, json!({}));
            assert_eq!(s.count(GameEventType::TrapFired), 1);
            s.expect_in_zone(&menace, "hand");
            assert!(s.hand(P1).iter().any(|card| card.id == menace.id));
            assert!(!s.hand(P2).iter().any(|card| card.id == menace.id));
        }

        #[test]
        fn r386_a_degrade_of_the_cost_makes_them_cost_1() {
            let mut s = wipe(false, json!([MENACE]), json!({}));
            step_param(s.card_mut(SHADOWSTEP), "setCost", 1);
            let menace = s.card(MENACE).clone();
            s.play(BIG_FELINOR, json!({}));
            assert_eq!(s.card(&menace).cost_override, Some(1));
        }

        /// R4395000: the trigger answers a Unit of yours that dies, not a Field Spell of yours destroyed.
        #[test]
        fn r4395000_a_field_spell_of_yours_destroyed_alone_leaves_it_set() {
            let mut s = datacenter();
            let program = s.card(FIELD_SPELL).clone();
            s.play(FIRE, json!({}));
            s.expect_in_zone(&program, "graveyard");
            assert_eq!(s.count(GameEventType::TrapFired), 0);
            assert_eq!(backrow_def(&s, P1, 2), Some(SHADOWSTEP.to_string()));
        }

        #[test]
        fn r4395000_a_trap_of_yours_destroyed_alone_leaves_it_set() {
            let mut s = walled(
                json!([{ "def": VANILLA, "lane": 3 }]),
                json!([
                    { "def": TRAP, "faceUp": false, "lane": 5 },
                    { "def": SHADOWSTEP, "faceUp": false, "lane": 3 },
                ]),
            );
            let trap = s.card(TRAP).clone();
            s.play(WALLS, json!({}));
            s.expect_in_zone(&trap, "graveyard");
            assert_eq!(s.count(GameEventType::TrapFired), 0);
            assert_eq!(backrow_def(&s, P1, 3), Some(SHADOWSTEP.to_string()));
            s.expect_in_zone(VANILLA, "field");
        }

        #[test]
        fn r4395000_a_field_spell_destroyed_in_the_same_pass_as_your_unit_stays_in_the_graveyard() {
            let mut s = walled(
                json!([{ "def": VANILLA, "lane": 1 }]),
                json!([
                    { "def": FIELD_SPELL, "lane": 5 },
                    { "def": SHADOWSTEP, "faceUp": false, "lane": 3 },
                ]),
            );
            let vanilla = s.card(VANILLA).clone();
            let program = s.card(FIELD_SPELL).clone();
            s.play(WALLS, json!({}));
            assert_eq!(s.count(GameEventType::TrapFired), 1);
            s.expect_in_zone(&vanilla, "hand");
            assert_eq!(s.card(&vanilla).cost_override, Some(0));
            s.expect_in_zone(&program, "graveyard");
            assert_eq!(def_ids(&s.hand(P1)), [STOCKPILE, VANILLA]);
        }

        /// R383, R4395001: an Animated Field Trap standing in a unit zone is a Unit, so its death answers it.
        #[test]
        fn r4395001_an_animated_field_trap_of_yours_that_dies_standing_as_a_unit_comes_back() {
            // Tesla fires on the Unit p2 plays and animates into lane 1; Crushing Walls then kills it there.
            let mut s = scenario(json!({
                "active": "p2",
                "p1": {
                    "hand": [STOCKPILE],
                    "library": [STOCKPILE],
                    "backrow": [
                        { "def": TESLA, "faceUp": false, "lane": 1 },
                        { "def": SHADOWSTEP, "faceUp": false, "lane": 3 },
                    ],
                },
                "p2": { "hand": [VANILLA, WALLS, STOCKPILE], "library": [STOCKPILE], "mana": 10 },
            }));
            let tesla = s.card(TESLA).clone();
            s.play(VANILLA, json!({ "zone": 2 }));
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(TESLA.to_string()));
            s.expect_in_zone(SHADOWSTEP, "field");
            s.play(WALLS, json!({}));
            s.expect_in_zone(SHADOWSTEP, "graveyard");
            s.expect_in_zone(&tesla, "hand");
            assert_eq!(s.card(&tesla).cost_override, Some(0));
        }

        /// R1081000: a Unit standing in the backrow is a Unit, whatever its zone.
        #[test]
        fn r4395000_a_unit_of_yours_standing_in_the_backrow_still_comes_back_beside_a_destroyed_field_spell() {
            let mut s = walled(
                json!([VANILLA]),
                json!([
                    { "def": FIELD_SPELL, "lane": 1 },
                    { "def": SHADOWSTEP, "faceUp": false, "lane": 3 },
                ]),
            );
            let vanilla = s.card(VANILLA).clone();
            let program = s.card(FIELD_SPELL).clone();
            crate::stand_in_backrow(&mut s, &vanilla.id, 5);
            s.play(WALLS, json!({}));
            assert_eq!(s.count(GameEventType::TrapFired), 1);
            s.expect_in_zone(&vanilla, "hand");
            assert_eq!(s.card(&vanilla).cost_override, Some(0));
            s.expect_in_zone(&program, "graveyard");
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
            assert_eq!(s.count(GameEventType::TrapFired), 1);
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
        fn r57_a_token_copy_keeps_stats_override_armor_override_and_tuning() {
            let mut s = wipe(true, json!([{ "def": RUSH_TOKEN }]), json!({}));
            let rush_id = s.card(RUSH_TOKEN).id.clone();
            s.card_mut(&rush_id).stats_override = Some(AttackHealth { attack: 4, health: 4 });
            s.card_mut(&rush_id).armor_override = Some(2);
            s.card_mut(&rush_id).tuning = Some(Tuning {
                attack: Some(1),
                ..Tuning::default()
            });
            s.play(BIG_FELINOR, json!({}));
            let copy = s.hand(P1).into_iter().find(|card| card.def_id == RUSH_TOKEN).expect("copy in hand");
            assert_eq!(copy.stats_override, Some(AttackHealth { attack: 4, health: 4 }));
            assert_eq!(copy.armor_override, Some(2));
            assert_eq!(copy.tuning, Some(Tuning { attack: Some(1), ..Tuning::default() }));
            assert_eq!(copy.cost_override, Some(0));
        }

        #[test]
        fn r1300_a_shadowstep_definition_copy_of_a_chinese_card_is_not_chinese() {
            let mut s = wipe(true, json!([VANILLA]), json!({}));
            let source = s.card(VANILLA).id.clone();
            s.card_mut(&source).chinese = Some(true);
            s.play(BIG_FELINOR, json!({}));
            assert_eq!(s.card(&source).chinese, Some(true), "the flickered card keeps its flag");
            let copy = copies(&s).into_iter().find(|card| card.def_id == VANILLA).expect("copy in hand");
            assert_eq!(copy.chinese, None, "R1300: Shadowstep remakes the copy from its definition");
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
            assert_eq!(s.count(GameEventType::TrapFired), 0);
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

        /// R4205000: nothing prevents the destroy of a card a steal had no room for, so no "would die"
        /// replacement is offered it: the Radiant flicker does not fire and stays set.
        #[test]
        fn r4205000_a_unit_a_steal_has_no_room_for_dies_past_the_radiant_flicker() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [MIND_CONTROL, STOCKPILE],
                    "mana": 6,
                    "field": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA],
                    "library": [STOCKPILE],
                },
                "p2": {
                    "hand": [STOCKPILE],
                    "field": [MENACE],
                    "backrow": [{ "def": SHADOWSTEP, "faceUp": false, "radiant": true }],
                    "library": [STOCKPILE, STOCKPILE],
                },
            }));
            let menace = s.card(MENACE).clone();
            s.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": menace.id }] }));
            assert_eq!(s.count(GameEventType::TrapFired), 0);
            s.expect_in_zone(&menace, "graveyard");
            assert!(s.pile(P2, "graveyard").iter().any(|card| card.id == menace.id));
            assert_eq!(backrow_def(&s, P2, 1), Some(SHADOWSTEP.to_string()));
        }
    }

    #[test]
    fn r767000_v0_4_12b_kept_script_uses_the_old_definition_copy() {
        crate::under_kept(SHADOWSTEP, script_v0_4_12b(), || {
            let mut s = wipe(true, json!([RUSH_TOKEN]), json!({}));
            let source = s.card(RUSH_TOKEN).id.clone();
            s.card_mut(&source).stats_override = Some(AttackHealth { attack: 4, health: 4 });
            s.card_mut(&source).armor_override = Some(2);
            s.play(BIG_FELINOR, json!({}));
            let copy = s.hand(P1).into_iter().find(|card| card.def_id == RUSH_TOKEN).expect("copy in hand");
            assert_eq!(copy.stats_override, None);
            assert_eq!(copy.armor_override, None);
        });
    }

    /// p1 sets a Cloaked Toe Cracker face-down beside its face-down base Shadowstep, and on p2's turn p2
    /// destroys it with Magic Jammed before it reveals; the set card's id.
    fn cloaked_jammed() -> (Scenario, String) {
        const MAGIC_JAMMED: &str = "core-036";
        let mut s = scenario(json!({
            "p1": {
                "hand": [crate::CLOAKED_UNIT, STOCKPILE],
                "library": [STOCKPILE, STOCKPILE],
                "backrow": [{ "def": SHADOWSTEP, "faceUp": false, "lane": 3 }],
                "mana": 10,
            },
            "p2": { "hand": [MAGIC_JAMMED, STOCKPILE], "library": [STOCKPILE, STOCKPILE] },
        }));
        let set = crate::set_cloaked(&mut s, P1, crate::CLOAKED_UNIT, 2);
        s.end_turn();
        s.play(MAGIC_JAMMED, json!({ "targets": pick::id(&set) }));
        (s, set)
    }

    /// R4423000: a Unit set face-down by its Cloaked and destroyed before it reveals died as a Field Trap,
    /// none of your Units, so the base face stays set and the card stays in your graveyard.
    #[test]
    fn r4423000_a_set_cloaked_unit_destroyed_before_it_reveals_is_no_unit_death_for_it() {
        let (s, set) = cloaked_jammed();
        assert_eq!(crate::died_as_of(&s, &set), Some(CardType::FieldTrap));
        assert_eq!(s.count(GameEventType::TrapFired), 0);
        assert_eq!(backrow_def(&s, P1, 3), Some(SHADOWSTEP.to_string()));
        assert!(s.pile(P1, "graveyard").iter().any(|card| card.id == set));
    }

    /// R767000: v0.6.9i's script read the set card by its definition's type, a Unit, and handed it back.
    #[test]
    fn r767000_the_script_kept_for_v0_6_9i_answers_a_set_cloaked_unit_s_death() {
        crate::under_kept(SHADOWSTEP, script_v0_6_9i(), || {
            let (s, set) = cloaked_jammed();
            assert_eq!(s.count(GameEventType::TrapFired), 1);
            let back = s.hand(P1).into_iter().find(|card| card.id == set).expect("the Toe Cracker in hand");
            assert_eq!(back.cost_override, Some(0));
        });
    }

    /// R4395000: v0.4.12b's and v0.6.9g's scripts answer every card of yours that was destroyed, a
    /// Field Spell included: they hand it back at cost 0 and are spent.
    #[test]
    fn r4395000_the_kept_scripts_still_answer_a_destroyed_field_spell_of_yours() {
        for (version, kept) in [("v0.4.12b", script_v0_4_12b()), ("v0.6.9g", script_v0_6_9g())] {
            crate::under_kept(SHADOWSTEP, kept, || {
                let mut s = datacenter();
                let program = s.card(FIELD_SPELL).clone();
                s.play(FIRE, json!({}));
                assert_eq!(s.count(GameEventType::TrapFired), 1, "{version}: it springs");
                s.expect_in_zone(&program, "hand");
                assert_eq!(s.card(&program).cost_override, Some(0), "{version}: at cost 0");
            });
        }
    }

    /// R1081000: a Unit of yours standing in the backrow that would die is one of your Units in the
    /// would-die window: the Radiant face flickers it where it stands.
    #[test]
    fn r1081000_radiant_flickers_your_unit_standing_in_the_backrow_where_it_stands() {
        let mut s = wipe(true, json!([VANILLA]), json!({}));
        let vanilla = s.card(VANILLA).id.clone();
        crate::stand_in_backrow(&mut s, &vanilla, 2);
        s.play(BIG_FELINOR, json!({}));
        assert_eq!(s.count(GameEventType::TrapFired), 1);
        assert_eq!(s.backrow(P1, 2).map(|card| card.id), Some(vanilla.clone()));
        assert!(stands_in_backrow(s.state(), s.card(vanilla.as_str())));
    }
}
