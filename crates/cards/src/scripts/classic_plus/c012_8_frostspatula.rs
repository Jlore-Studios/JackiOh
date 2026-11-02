//! C+ #12.8 Frostspatula (SPEC §8.7 row 12.8, R409): (2) Field Spell, Pancake, Token (printed Legendary),
//! 7/3 → 14/6.
//!   Base:    "Animated on your turn, Rush. Death: Summon a copy of every Unit this destroyed."
//!   Radiant: "… and make them Radiant."
//! Animated on your turn and Rush are catalog keywords the engine runs (B3.1, R383). It remembers each
//! Unit it killed (R42's killer) by definition and face, in memory, which its animations keep (R383);
//! its Death — as a Unit or destroyed in the backrow — summons a fresh copy of each for its controller,
//! tokens included, until the board is full, with no Cry (R1). The originals stay put (R409).

use jackioh_engine::effects::{remember, summon};
use jackioh_engine::prelude::*;
use serde::{Deserialize, Serialize};

pub const ID: &str = "classicplus-012-8";

const KILLS: &str = "kills";

/// One remembered kill, as it rides the card's memory (plain JSON, R383).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Kill {
    id: String,
    def_id: String,
    radiant: bool,
}

/// The kills this card remembers; anything but a list reads as none.
fn kills_of(ctx: &EffectContext) -> Vec<Kill> {
    kills_in(recalled(ctx, KILLS).as_ref())
}

/// The kills as `memory.kills` stores them: what Death summons copies of and the preview counts.
fn kills_in(stored: Option<&Value>) -> Vec<Kill> {
    match stored {
        Some(Value::Array(kills)) => kills
            .iter()
            .filter_map(|kill| serde_json::from_value::<Kill>(kill.clone()).ok())
            .collect(),
        _ => Vec::new(),
    }
}

/// R2476000: "every Unit this destroyed" — how many it remembers, on the field to both seats (each kill
/// was a public destruction; R78 forgets them as it leaves). A kill in the pass of its own death is not
/// remembered yet (its Death reads it off that pass's events), which no value asked now can foresee.
fn preview() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        if ctx.zone != ConditionZone::Field {
            return vec![];
        }
        vec![count_preview("every Unit this destroyed", kills_in(ctx.self_.memory.get(KILLS)).len() as i32)]
    })
}

/// A `destroyed` event this card is the killer of, as a remembered kill.
fn kill_in(ctx: &EffectContext, event: &GameEvent) -> Option<Kill> {
    let GameEvent::Destroyed {
        instance_id,
        def_id,
        killer_id,
        radiant,
        ..
    } = event
    else {
        return None;
    };
    let me = ctx.self_.as_ref()?;
    if killer_id.as_deref() != Some(me.id.as_str()) {
        return None;
    }
    Some(Kill {
        id: instance_id.clone(),
        def_id: def_id.clone(),
        radiant: *radiant == Some(true),
    })
}

fn frostspatula(radiant: bool) -> Script {
    Script {
        triggers: vec![TriggerDef::new(
            "frostspatula-kill",
            &[GameEventType::Destroyed],
            // The condition is read in `run`, as a `when` would be for a trigger that is no trap (R2271000).
            |ctx, event| {
                let view: &EffectContext = ctx;
                match kill_in(view, event) {
                    None => Vec::new(),
                    Some(kill) => {
                        let mut kills = kills_of(view);
                        kills.push(kill);
                        vec![remember(json_as(json!({ "key": KILLS, "value": kills })))]
                    }
                }
            },
        )],
        preview: Some(preview()),
        death: Some(hook(move |ctx| {
            let view: &EffectContext = ctx;
            // A kill in the same pass as its own death never reached the trigger: read it off this action's events.
            let known = kills_of(view);
            let late: Vec<Kill> = view
                .events
                .iter()
                .filter_map(|event| kill_in(view, event))
                .filter(|kill| !known.iter().any(|seen| seen.id == kill.id))
                .collect();
            known
                .iter()
                .chain(late.iter())
                .map(|kill| summon(json_as(json!({ "defId": kill.def_id, "radiant": radiant || kill.radiant }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: frostspatula(false),
        radiant: frostspatula(true),
    }
}

// C+ #12.8 Frostspatula — SPEC §8.7 row 12.8, BUILD M9 Classic+ row C+ 12.8, R383, R409, R1062; both
// moves keep damage, buffs and memory (R78 does not apply); its Death fires in the backrow too (§4.5).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SPATULA: &str = "classicplus-012-8";
    const TOKENS: &str = "core-015"; // (1) 1/1, Cry: summon a Rush Token
    const MENACE: &str = "core-019"; // 9/9
    const RUSH: &str = "core-t-rush"; // 3/3 Rush
    const MAGIC_JAMMED: &str = "core-036"; // (1) destroy target backrow card
    const POWDER: &str = "classicplus-012-4"; // (1) 3 damage to each enemy
    const MOTHER: &str = "classicplus-012"; // End of turn: add a random Pancake token to your hand
    const SURGERY: &str = "core-063"; // (1) +3/+3 and 1 random keyword
    const WASTES: &str = "classicplus-012-6"; // (2) Field Spell, Cry: destroy all Units
    const FIENDER: &str = "core-092"; // (2) 5/7 Stack
    const MROW: &str = "core-086"; // 1/1 Can't attack; Death: take control of the Unit that destroyed this
    const MANA_WELL: &str = "core-006"; // (3) Field Spell
    const FILLER: &str = "core-005";
    const DECK: [&str; 6] = [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER];

    use crate::scenario;

    use crate::js;

    use crate::merged;

    use crate::matches_object;

    fn played(radiant: bool, lane: i32, p1: Value, p2: Value) -> Scenario {
        let mut s = scenario(json!({
            "p1": merged(json!({ "hand": [{ "def": SPATULA, "radiant": radiant }, FILLER, FILLER], "library": DECK, "mana": 8 }), p1),
            "p2": merged(json!({ "hand": [FILLER], "library": DECK, "field": [TOKENS] }), p2),
        }));
        s.play(SPATULA, json!({ "zone": lane }));
        s
    }

    fn spatula_id(s: &Scenario) -> String {
        s.card(SPATULA).id.clone()
    }

    fn count_of(events: &[GameEvent], kind: GameEventType) -> usize {
        events.iter().filter(|event| event.event_type() == kind).count()
    }

    mod base {
        use super::*;

        /// R383 played on your turn it animates at once into its own lane's unit zone, its backrow zone reserved
        #[test]
        fn r383_played_on_your_turn_it_animates_at_once_into_its_own_lanes_unit_zone_its_backrow_zone_reserved() {
            let mut s = played(false, 2, json!({}), json!({}));
            assert_eq!(s.unit(P1, 2).map(|card| card.id), Some(spatula_id(&s)));
            assert_eq!(js(&s.view(P1))["you"]["reserved"]["backrow"][1], json!(true));
            assert_eq!(count_of(s.events(), GameEventType::Animated), 1);
            assert!(s.view(P2).events.iter().any(|event| event.event_type() == GameEventType::Animated));
            s.expect_stats(SPATULA, json!({ "attack": 7, "health": 3 }));
        }

        /// R383 with its lane's unit zone taken it goes to the leftmost open one; with none it stays in the backrow
        #[test]
        fn r383_with_its_lanes_unit_zone_taken_it_goes_to_the_leftmost_open_one_with_none_it_stays_in_the_backrow() {
            let taken = played(false, 2, json!({ "field": [{ "def": MENACE, "lane": 2 }] }), json!({}));
            assert_eq!(taken.unit(P1, 1).map(|card| card.id), Some(spatula_id(&taken)));
            let full = played(false, 2, json!({ "field": [MENACE, MENACE, MENACE, MENACE, MENACE] }), json!({}));
            assert_eq!(full.backrow(P1, 2).map(|card| card.id), Some(spatula_id(&full)));
        }

        /// R2476000: after "every Unit this destroyed", how many it remembers, to both seats: 0, then 1
        /// once its Rush attack destroys a Unit.
        #[test]
        fn r2476000_it_previews_the_units_it_has_destroyed() {
            for radiant in [false, true] {
                let mut s = played(radiant, 1, json!({}), json!({}));
                let spatula = spatula_id(&s);
                let label = "every Unit this destroyed".to_string();
                assert_eq!(crate::preview_seen(&s, P1, &spatula), vec![(label.clone(), 0, None)], "radiant {radiant}");
                s.attack(SPATULA, TOKENS);
                assert!(s.pile(P2, "graveyard").iter().any(|card| card.def_id == TOKENS));
                assert_eq!(crate::preview_seen(&s, P1, &spatula), vec![(label.clone(), 1, None)], "radiant {radiant}");
                assert_eq!(crate::preview_seen(&s, P2, &spatula), vec![(label, 1, None)], "radiant {radiant}");
            }
        }

        /// R83 summoning sick on arrival: its Rush reaches units, never the hero
        #[test]
        fn r83_summoning_sick_on_arrival_its_rush_reaches_units_never_the_hero() {
            let mut s = played(false, 1, json!({}), json!({}));
            s.expect_refused(|s| s.attack(SPATULA, "hero"));
            s.attack(SPATULA, TOKENS);
            assert!(s.pile(P2, "graveyard").iter().any(|card| card.def_id == TOKENS));
        }

        /// R383 it returns to its backrow zone at your cleanup, keeping its damage; on the opponent's turn it hides there
        #[test]
        fn r383_it_returns_to_its_backrow_zone_at_your_cleanup_keeping_its_damage_on_the_opponents_turn_it_hides_there() {
            let mut s = played(false, 2, json!({}), json!({ "hand": [POWDER, FILLER] }));
            s.attack(SPATULA, TOKENS); // takes 1 damage
            s.end_turn();
            assert_eq!(s.backrow(P1, 2).map(|card| card.id), Some(spatula_id(&s)));
            assert!(s.unit(P1, 2).is_none());
            assert_eq!(count_of(s.events(), GameEventType::Deanimated), 1);
            // The opponent can't attack it, and their "all Units" sweep skips it.
            s.play(POWDER, json!({}));
            assert!(matches_object(&js(&s.card(SPATULA).zone), &json!({ "z": "field", "row": "backrow" })));
            assert_eq!(s.card(SPATULA).damage, 1);
            s.end_turn();
            assert_eq!(s.unit(P1, 2).map(|card| card.id), Some(spatula_id(&s)));
            s.expect_stats(SPATULA, json!({ "health": 2 }));
        }

        /// R383 R688 a home zone Locked meanwhile still takes the return at cleanup: a Lock refuses plays, not the return
        #[test]
        fn r383_r688_a_home_zone_locked_meanwhile_still_takes_the_return_at_cleanup_a_lock_refuses_plays_not_the_return() {
            let mut s = played(false, 2, json!({}), json!({}));
            // A Lock on its reserved backrow zone, as Lock effects leave one (§3.2).
            lock_zone(
                s.state_mut(),
                ZoneSlot {
                    player: P1,
                    row: Row::Backrow,
                    lane: 2,
                },
            );
            s.end_turn();
            assert_eq!(s.backrow(P1, 2).map(|card| card.id), Some(spatula_id(&s)));
            assert!(s.unit(P1, 2).is_none());
            assert_eq!(count_of(s.events(), GameEventType::Deanimated), 1);
        }

        /// backrow effects reach it on the opponent's turn
        #[test]
        fn backrow_effects_reach_it_on_the_opponents_turn() {
            let mut s = played(false, 2, json!({}), json!({ "hand": [MAGIC_JAMMED, FILLER] }));
            s.end_turn();
            let id = spatula_id(&s);
            s.play(MAGIC_JAMMED, json!({ "targets": pick::id(&id) }));
            s.expect_in_zone(SPATULA, "graveyard");
        }

        #[test]
        fn r409_it_remembers_each_unit_it_killed_and_its_death_summons_a_fresh_copy_of_each_for_you_with_no_cry() {
            let mut s = played(false, 1, json!({}), json!({ "hand": [MAGIC_JAMMED, FILLER] }));
            let victim = s.card(TOKENS).id.clone();
            s.attack(SPATULA, TOKENS);
            let kills = s.card(SPATULA).memory.get("kills").cloned().unwrap_or(Value::Null);
            assert!(matches_object(&kills, &json!([{ "defId": TOKENS, "radiant": false }])));
            s.end_turn();
            let id = spatula_id(&s);
            s.play(MAGIC_JAMMED, json!({ "targets": pick::id(&id) }));
            // Destroyed in the backrow: its Death fires. A fresh Me and Mr Token for p1, no Cry (no Rush Token).
            let copies: Vec<CardInstance> = (1..=5)
                .filter_map(|lane| s.unit(P1, lane))
                .filter(|unit| unit.def_id == TOKENS)
                .collect();
            assert_eq!(copies.len(), 1);
            assert_ne!(copies[0].id, victim);
            assert_eq!(s.card(&victim).zone.z(), ZoneName::Graveyard);
            assert_eq!(s.card(&victim).owner, P2);
            assert!(
                !s.events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == RUSH))
            );
        }

        /// R409 R572 tokens included, and a kill in the same pass as its own death counts
        #[test]
        fn r409_r572_tokens_included_and_a_kill_in_the_same_pass_as_its_own_death_counts() {
            let mut s = played(false, 1, json!({}), json!({ "field": [RUSH] }));
            let id = spatula_id(&s);
            s.attack(SPATULA, RUSH); // 7 kills the 3/3; the 3/3 kills the 7/3
            // A token that dies as a Unit ceases to exist (R11).
            s.expect_in_zone(&id, "gone");
            assert_eq!(s.unit(P1, 1).map(|card| card.def_id).as_deref(), Some(RUSH));
        }

        #[test]
        fn r409_the_copies_go_in_until_your_board_is_full() {
            let mut s = played(
                false,
                1,
                json!({ "field": [{ "def": MENACE, "lane": 2 }, { "def": MENACE, "lane": 3 }, { "def": MENACE, "lane": 4 }, { "def": MENACE, "lane": 5 }] }),
                json!({ "field": [RUSH] }),
            );
            s.attack(SPATULA, RUSH);
            // Its own zone opened as it died: one copy fits.
            assert_eq!(s.unit(P1, 1).map(|card| card.def_id).as_deref(), Some(RUSH));
        }

        #[test]
        fn its_kills_survive_its_moves_between_the_rows_r383_memory_is_kept() {
            let mut s = played(false, 1, json!({}), json!({ "field": [TOKENS, TOKENS] }));
            let first = s.unit(P2, 1).map(|card| card.id).unwrap_or_default();
            s.attack(SPATULA, &first);
            s.end_turn().end_turn();
            let second = s.unit(P2, 2).map(|card| card.id).unwrap_or_default();
            s.attack(SPATULA, &second);
            assert_eq!(
                s.card(SPATULA).memory.get("kills").and_then(Value::as_array).map(Vec::len),
                Some(2)
            );
        }

        #[test]
        fn r1062_r383_animated_again_at_your_next_start_of_turn_it_is_not_summoning_sick_and_its_rush_reaches_the_hero() {
            let mut s = played(false, 1, json!({}), json!({}));
            s.expect_refused(|s| s.attack(SPATULA, "hero"));
            s.expect_health(P2, HERO_HEALTH);
            s.end_turn().end_turn();
            assert_eq!(s.unit(P1, 1).map(|card| card.id), Some(spatula_id(&s)));
            s.attack(SPATULA, "hero");
            s.expect_health(P2, HERO_HEALTH - 7);
        }

        /// §2.2 R383 it returns after your end-of-turn steps, and animates after your mana refresh, before your draw
        #[test]
        fn s2_2_r383_it_returns_after_your_end_of_turn_steps_and_animates_after_your_mana_refresh_before_your_draw() {
            let mut s = played(false, 2, json!({ "field": [MOTHER] }), json!({}));
            s.end_turn();
            let added = s
                .events()
                .iter()
                .position(|event| matches!(event, GameEvent::AddedToHand { player, .. } if *player == P1))
                .expect("an addedToHand for p1");
            let home = s
                .events()
                .iter()
                .position(|event| event.event_type() == GameEventType::Deanimated)
                .expect("a deanimated");
            assert!(home > added);
            assert!(s.view(P2).events.iter().any(|event| event.event_type() == GameEventType::Deanimated));

            s.end_turn();
            let start = s.last_events().to_vec();
            let refresh = start
                .iter()
                .position(|event| matches!(event, GameEvent::ManaChanged { player, .. } if *player == P1))
                .expect("a manaChanged for p1");
            let animated = start
                .iter()
                .position(|event| event.event_type() == GameEventType::Animated)
                .expect("an animated");
            let drawn = start
                .iter()
                .position(|event| event.event_type() == GameEventType::Drawn)
                .expect("a drawn");
            assert!(animated > refresh);
            assert!(drawn > animated);
        }

        #[test]
        fn r383_both_moves_keep_its_buffs() {
            let mut s = played(false, 2, json!({ "hand": [{ "def": SPATULA }, SURGERY, FILLER] }), json!({}));
            let id = spatula_id(&s);
            s.play(SURGERY, json!({ "targets": pick::id(&id) }));
            s.expect_stats(SPATULA, json!({ "attack": 10, "maxHealth": 6 }));
            s.end_turn();
            assert_eq!(s.backrow(P1, 2).map(|card| card.id), Some(spatula_id(&s)));
            s.end_turn();
            assert_eq!(s.unit(P1, 2).map(|card| card.id), Some(spatula_id(&s)));
            s.expect_stats(SPATULA, json!({ "attack": 10, "maxHealth": 6 }));
        }

        /// R383 on the opponent's turn an 'all Units' destroy skips it in the backrow
        #[test]
        fn r383_on_the_opponents_turn_an_all_units_destroy_skips_it_in_the_backrow() {
            let mut s = played(false, 2, json!({}), json!({ "hand": [WASTES, FILLER], "mana": 8 }));
            s.end_turn();
            s.play(WASTES, json!({}));
            assert!(matches_object(
                &js(&s.card(SPATULA).zone),
                &json!({ "z": "field", "row": "backrow", "lane": 2 })
            ));
            s.expect_in_zone(TOKENS, "graveyard");
        }

        #[test]
        fn r383_a_stack_over_it_keeps_it_a_unit_dormant_through_your_cleanup_its_backrow_zone_still_held() {
            let mut s = played(false, 2, json!({ "hand": [{ "def": SPATULA }, FIENDER, FILLER] }), json!({}));
            s.play(FIENDER, json!({ "zone": 2 }));
            assert_eq!(s.unit(P1, 2).map(|card| card.def_id).as_deref(), Some(FIENDER));
            s.end_turn();
            assert!(matches_object(
                &js(&s.card(SPATULA).zone),
                &json!({ "z": "field", "row": "units", "lane": 2 })
            ));
            assert!(s.backrow(P1, 2).is_none());
            assert_eq!(js(&s.view(P1))["you"]["reserved"]["backrow"][1], json!(true));
            assert_eq!(count_of(s.events(), GameEventType::Deanimated), 0);
        }

        #[test]
        fn r383_a_new_controller_keeps_it_a_unit_until_its_own_cleanup_then_it_goes_to_their_leftmost_open_backrow_zone() {
            // It kills "Miss" Mrow, whose Death hands it to p2 on p1's turn.
            let mut s = played(
                false,
                2,
                json!({}),
                json!({ "field": [{ "def": MROW, "lane": 3 }], "backrow": [{ "def": MANA_WELL, "lane": 1 }] }),
            );
            s.attack(SPATULA, MROW);
            assert_eq!(s.card(SPATULA).controller, P2);
            s.end_turn(); // p1's cleanup: not its controller's, so it stays a Unit
            assert!(matches_object(
                &js(&s.card(SPATULA).zone),
                &json!({ "z": "field", "player": "p2", "row": "units" })
            ));
            s.end_turn(); // p2's cleanup: no home on p2's side, so p2's leftmost open backrow zone
            assert_eq!(s.backrow(P2, 2).map(|card| card.id), Some(spatula_id(&s)));
            assert_eq!(js(&s.view(P1))["you"]["reserved"]["backrow"][1], json!(false));
        }
    }

    mod radiant {
        use super::*;

        /// 14/6, and the copies are Radiant
        #[test]
        fn fourteen_6_and_the_copies_are_radiant() {
            let mut s = played(true, 1, json!({}), json!({ "hand": [MAGIC_JAMMED, FILLER] }));
            s.expect_stats(SPATULA, json!({ "attack": 14, "health": 6 }));
            s.attack(SPATULA, TOKENS);
            s.end_turn();
            let id = spatula_id(&s);
            s.play(MAGIC_JAMMED, json!({ "targets": pick::id(&id) }));
            let copy = (1..=5).filter_map(|lane| s.unit(P1, lane)).find(|unit| unit.def_id == TOKENS);
            assert_eq!(copy.map(|unit| unit.radiant), Some(true));
        }
    }
}
