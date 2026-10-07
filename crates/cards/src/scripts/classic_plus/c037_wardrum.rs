//! C+ #37 Wardrum (SPEC §8.7 row 37): (5) Unit, Quickdraw, Legendary, 5/5 → 10/10.
//!   Base:    "While this is in your hand or deck: After the Spells, Field Spells and Traps you play in a
//!            turn reach {threshold}, summon this. End of turn: Cast a copy of a random Spell, Field Spell
//!            or Trap you played this turn."
//!   Radiant: "… End of turn: Cast a copy of each Spell, Field Spell and Trap you played this turn."
//! A hand and deck trigger (B5 E26) on the resolution of the play that is the threshold-th non-Unit play
//! of the turn (casts count, R70); `summonThis` puts it in the leftmost open unit zone with no Cry, or
//! leaves it where it is. The end of turn casts fresh copies by definition and face (E12, R87), skipping
//! played cards that no longer exist (R86).

use jackioh_engine::effects::{
    CastDef, CastHow, CastNewArgs, CastNewDef, ForEachCardArgs, cast_new, for_each_card, summon_this,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-037";

const NON_UNIT: &[CardType] = &[CardType::Spell, CardType::FieldSpell, CardType::Trap, CardType::FieldTrap];

/// The type a card was played as: its running face's, wherever it stands now (B2.7).
fn non_unit(state: &GameState, card: &CardInstance) -> bool {
    NON_UNIT.contains(&card_type_of_face(state, &card.def_id, card.radiant))
}

/// This turn's non-Unit plays of `player` that still exist, in play order (R86).
fn non_unit_plays(state: &GameState, player: PlayerId) -> Vec<CardInstance> {
    played_ids_this_turn(state, player)
        .iter()
        .filter_map(|id| find_instance(state, id))
        .filter(|card| non_unit(state, card))
        .cloned()
        .collect()
}

/// R578: whether the play this event resolves is the controller's threshold-th non-Unit play of the turn.
/// Its place is the turn's count of such plays (which keeps a play whose card has since ceased to exist)
/// less those logged after its latest play — the casts its own resolution made — so a 4th cast inside the
/// 3rd's resolution answers neither as the 3rd, and a card played again this turn is placed by this play.
fn reaches_threshold(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::CardResolved { player, instance_id, def_id, radiant, .. } = event else {
        return false;
    };
    if *player != ctx.controller {
        return false;
    }
    let state: &GameState = ctx.state;
    if !NON_UNIT.contains(&card_type_of_face(state, def_id, *radiant == Some(true))) {
        return false;
    }
    let log = played_ids_this_turn(state, ctx.controller);
    let Some(at) = log.iter().rposition(|id| id == instance_id) else {
        return false;
    };
    // ponytail: a cast inside this resolution whose card has ceased to exist since is not taken off.
    let later = log[at + 1..]
        .iter()
        .filter(|id| find_instance(state, id).is_some_and(|card| non_unit(state, card)))
        .count() as i32;
    played_this_turn_of_type(state, ctx.controller, NON_UNIT) - later == param(ctx, "threshold")
}

fn summon_at_threshold() -> TriggerDef {
    // Not a trap, so the condition is read in `run` (only traps consult `when`, R99).
    TriggerDef::new("wardrum-summon", &[GameEventType::CardResolved], |ctx, event| {
        if reaches_threshold(ctx, event) { vec![summon_this()] } else { vec![] }
    })
}

/// R195: in hand, on your turn, your next Spell, Field Spell or Trap would summon it into an open zone.
fn next_play_summons(c: ConditionContext<'_>) -> bool {
    if c.zone != ConditionZone::Hand || !c.your_turn {
        return false;
    }
    let threshold = param(&c, "threshold");
    played_this_turn_of_type(c.state, c.controller, NON_UNIT) == threshold - 1
        && first_free_zone(c.state, c.controller, Row::Units).is_some()
}

/// `castNew({ def })` with the definition read off the context as the effect applies (part 6.2's
/// `CastNewArgs`: its `def` is `CastNewDef::Read`), cast with no special `how`.
fn cast_new_read(read: impl Fn(&mut EffectContext<'_>) -> Option<CastDef> + Send + Sync + 'static) -> Effect {
    cast_new(CastNewArgs {
        def: CastNewDef::Read(Arc::new(read)),
        radiant: None,
        how: CastHow::default(),
    })
}

fn copy_of(id: String) -> impl Fn(&mut EffectContext<'_>) -> Option<CastDef> + Send + Sync + 'static {
    move |ctx| {
        find_instance(ctx.state, &id).map(|card| CastDef {
            def_id: card.def_id.clone(),
            radiant: Some(card.radiant),
        })
    }
}

/// The radiant face's `forEachCard` list: this turn's non-Unit plays that still exist, by id.
fn played_non_unit_ids(ctx: &mut EffectContext<'_>) -> Vec<String> {
    non_unit_plays(ctx.state, ctx.controller).into_iter().map(|card| card.id).collect()
}

/// The radiant face's `forEachCard` step: cast a copy of that card, by definition and face.
fn cast_copy_of(id: &str) -> Effect {
    cast_new_read(copy_of(id.to_string()))
}

fn wardrum(end_of_turn: Hook) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        hand_triggers: vec![summon_at_threshold()],
        deck_triggers: vec![summon_at_threshold()],
        condition_met: Some(condition_hook(next_play_summons)),
        end_of_turn: Some(end_of_turn),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = wardrum(hook(|_ctx| {
        vec![cast_new_read(|ctx| {
            let plays = non_unit_plays(ctx.state, ctx.controller);
            ctx.rng.pick(&plays).map(|card| CastDef {
                def_id: card.def_id.clone(),
                radiant: Some(card.radiant),
            })
        })]
    }));

    let radiant = wardrum(hook(|_ctx| {
        vec![for_each_card(ForEachCardArgs {
            cards: Arc::new(played_non_unit_ids),
            each: Arc::new(cast_copy_of),
        })]
    }));

    CardScripts { base, radiant }
}

// C+ #37 Wardrum — SPEC §8.7 row 37, BUILD M9 Classic+ row C+ 37: "Quickdraw; while in your hand or
// deck, once your 3rd Spell, Field Spell or Trap play of a turn has resolved (casts count, R70; Units
// don't; the 4th doesn't), it is summoned into your leftmost open unit zone with no Cry (R1), from the
// deck too after a mulligan returned it; with no open zone it stays where it is; on the field, at the
// end of your turn, it casts a copy (R70) of one random Spell, Field Spell or Trap you played this
// turn, by definition and face, a Trap copy set face-down and fizzling to the graveyard with no open
// backrow zone; none played, nothing; a Trap copy stays hidden from the opponent (R33, R97), and a
// hand or deck trigger that does not fire shows the opponent nothing, the summon being the first they
// see of it; `conditionMet` in hand answers whether your next such play would summon it (R195); the
// threshold reads through `param()` and never drops below 2; radiant casts a copy of each".
//
// The R195 proofs live in this file (both answers, against the branch the play then takes).
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const WARDRUM: &str = "classicplus-037";
    const REPLENISH: &str = "core-010"; // (0) Spell: Combo 3: draw 3.
    const STOCKPILE: &str = "core-005"; // (1) Spell: draw 2, heal your hero 2.
    const LUNAR: &str = "core-035"; // (1) Spell: 3 damage to a target.
    const BONE_STORM: &str = "classicplus-036-1"; // (1) Spell: 1 damage to each enemy.
    const SHEEPISH: &str = "core-041"; // (1) Trap.
    const BREAD: &str = "core-018"; // (1) Field Trap.
    const MANA_WELL: &str = "core-006"; // (3) Field Spell.
    const VANILLA: &str = "core-008"; // (1) Unit, no text.
    const MENACE: &str = "core-019";
    const POINTMASTER: &str = "core-020"; // (2) 7/1.
    const TIMMY: &str = "core-011"; // (1) 3/3 Rush, First Strike.
    const FOREVER: &str = "classicplus-014"; // (1) Spell: the next Spell you play returns to your hand after it resolves.
    const SPATULA: &str = "classicplus-012-8"; // (2) Field Spell token: Animated on your turn, Rush, 10/3.
    const FILLER: &str = STOCKPILE;

    const LIBRARY: [&str; 8] = [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER];

    /// The harness's `scenario`, with the shipped cards registered first (the TS harness did it at import).
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    fn hand(s: &Scenario, def_id: &str) -> String {
        match s.hand(P1).into_iter().find(|held| held.def_id == def_id) {
            Some(card) => card.id,
            None => panic!("no {def_id} in p1's hand"),
        }
    }

    fn play_three(s: &mut Scenario) -> &mut Scenario {
        let first = hand(s, REPLENISH);
        s.play(&first, json!({}));
        let second = hand(s, REPLENISH);
        s.play(&second, json!({}));
        let third = hand(s, REPLENISH);
        s.play(&third, json!({}));
        s
    }

    /// Play p1's hand card of this definition.
    fn play_from_hand(s: &mut Scenario, def_id: &str, opts: Value) {
        let id = hand(s, def_id);
        s.play(&id, opts);
    }

    fn wardrum_in(s: &Scenario) -> ZoneName {
        s.card(WARDRUM).zone.z()
    }

    fn view(s: &Scenario, seat: PlayerId) -> Value {
        serde_json::to_value(s.view(seat)).expect("a view is JSON")
    }

    fn view_text(s: &Scenario, seat: PlayerId) -> String {
        serde_json::to_string(&s.view(seat)).expect("a view is JSON")
    }

    fn events_json(s: &Scenario) -> Vec<Value> {
        s.events().iter().map(|event| serde_json::to_value(event).expect("an event is JSON")).collect()
    }

    fn glows(s: &Scenario) -> bool {
        let own = view(s, P1)["you"]["hand"].clone();
        let Some(cards) = own.as_array() else {
            panic!("own hand in full");
        };
        cards
            .iter()
            .find(|card| card["defId"] == WARDRUM)
            .is_some_and(|card| card["conditionActive"] == true)
    }

    /// TS `Array.prototype.indexOf`: the first position, or -1.
    fn index_of(list: &[String], item: &str) -> i64 {
        list.iter().position(|entry| entry == item).map_or(-1, |at| at as i64)
    }

    fn at_hero(player: &str) -> Value {
        json!({ "targets": [{ "pick": "hero", "player": player }] })
    }

    #[test]
    fn is_tagged_quickdraw_and_acclaimed_spec_8_7_row_37_patch_v0_2_y() {
        assert_eq!(crate::card_def(WARDRUM).tags, vec![Tag::Quickdraw, Tag::Acclaimed]);
    }

    mod base {
        use super::*;

        #[test]
        fn quickdraw_dealt_a_deck_holding_it_it_starts_in_the_opening_hand_2_1() {
            crate::register_all();
            let others: Vec<String> = query(&CatalogQueryArgs::default())
                .iter()
                .map(|def| def.id.clone())
                .filter(|id| id != WARDRUM)
                .take((DECK_SIZE - 1) as usize)
                .collect();
            let mut deck = others;
            deck.push(WARDRUM.to_string());
            for seed in ["wd-q1", "wd-q2", "wd-q3"] {
                let options: CreateGameOptions = json_as(json!({ "seed": seed, "decks": [deck, deck] }));
                let begun = begin_game(&create_game(&options)).state;
                assert!(begun.players.p1.hand.iter().any(|card| card.def_id == WARDRUM), "{seed}");
                assert!(begun.players.p2.hand.iter().any(|card| card.def_id == WARDRUM), "{seed}");
            }
        }

        #[test]
        fn in_hand_once_the_3rd_spell_of_the_turn_has_resolved_it_is_summoned_into_the_leftmost_open_zone() {
            let mut s = scenario(json!({
                "p1": { "hand": [WARDRUM, REPLENISH, REPLENISH, REPLENISH], "field": [{ "def": MENACE, "lane": 1 }], "library": LIBRARY },
                "p2": { "hand": [FILLER] },
            }));
            play_from_hand(&mut s, REPLENISH, json!({}));
            play_from_hand(&mut s, REPLENISH, json!({}));
            assert_eq!(wardrum_in(&s), ZoneName::Hand);
            play_from_hand(&mut s, REPLENISH, json!({}));
            assert_eq!(s.unit(P1, 2).map(|card| card.def_id), Some(WARDRUM.to_string()));
            // Summoned after the 3rd resolved: its draw-3 came first.
            let events = events_json(&s);
            let resolved = events.iter().rposition(|event| event["type"] == "cardResolved").map_or(-1, |at| at as i64);
            let summoned = events
                .iter()
                .position(|event| event["type"] == "summoned" && event["defId"] == WARDRUM)
                .map_or(-1, |at| at as i64);
            assert!(summoned > resolved);
            assert!(!events.iter().any(|event| event["type"] == "cardPlayed" && event["defId"] == WARDRUM));
        }

        #[test]
        fn field_spells_traps_and_field_traps_count_units_do_not() {
            let mut s = scenario(json!({
                "p1": { "hand": [WARDRUM, VANILLA, VANILLA, SHEEPISH, BREAD, MANA_WELL, FILLER], "library": LIBRARY, "mana": 10 },
                "p2": { "hand": [FILLER] },
            }));
            play_from_hand(&mut s, VANILLA, json!({}));
            play_from_hand(&mut s, VANILLA, json!({}));
            play_from_hand(&mut s, SHEEPISH, json!({}));
            play_from_hand(&mut s, BREAD, json!({}));
            assert_eq!(wardrum_in(&s), ZoneName::Hand);
            play_from_hand(&mut s, MANA_WELL, json!({}));
            assert_eq!(wardrum_in(&s), ZoneName::Field);
        }

        #[test]
        fn r70_casts_count_a_bone_storm_cast_on_draw_is_one_of_the_three() {
            let mut s = scenario(json!({
                "p1": { "hand": [WARDRUM, REPLENISH, REPLENISH, STOCKPILE], "library": [BONE_STORM, FILLER, FILLER, FILLER], "mana": 4 },
                "p2": { "hand": [FILLER] },
            }));
            play_from_hand(&mut s, REPLENISH, json!({}));
            play_from_hand(&mut s, STOCKPILE, json!({})); // draws the Bone Storm: it casts itself (the 3rd), then draws again.
            assert_eq!(wardrum_in(&s), ZoneName::Field);
        }

        #[test]
        fn r578_a_cast_inside_the_3rd_play_is_the_4th_it_fires_on_the_3rds_resolution_after_the_casts() {
            let mut s = scenario(json!({
                "p1": { "hand": [WARDRUM, REPLENISH, REPLENISH, STOCKPILE], "library": [BONE_STORM, FILLER, FILLER, FILLER], "mana": 4 },
                "p2": { "hand": [FILLER] },
            }));
            play_from_hand(&mut s, REPLENISH, json!({}));
            play_from_hand(&mut s, REPLENISH, json!({}));
            let stockpile = hand(&s, STOCKPILE);
            s.play(&stockpile, json!({})); // its draw casts the Bone Storm (the 4th) before the Stockpile (the 3rd) resolves
            assert_eq!(wardrum_in(&s), ZoneName::Field);
            let order: Vec<String> = events_json(&s)
                .iter()
                .filter_map(|event| {
                    if event["type"] == "cardResolved" {
                        event["instanceId"].as_str().map(str::to_string)
                    } else if event["type"] == "summoned" && event["defId"] == WARDRUM {
                        Some("wardrum".to_string())
                    } else {
                        None
                    }
                })
                .collect();
            assert!(index_of(&order, "wardrum") > index_of(&order, &stockpile));
        }

        #[test]
        fn r578_a_spell_played_again_this_turn_is_counted_at_each_play_forever_and_s_returning_spell_played_a_2nd_time_is_the_3rd() {
            let mut s = scenario(json!({
                "p1": { "hand": [WARDRUM, FOREVER, LUNAR, FILLER], "library": LIBRARY, "mana": 10 },
                "p2": { "hand": [FILLER], "library": LIBRARY },
            }));
            play_from_hand(&mut s, FOREVER, json!({})); // the 1st
            let lunar = hand(&s, LUNAR);
            s.play(&lunar, at_hero("p2")); // the 2nd; it comes back to hand
            assert_eq!(wardrum_in(&s), ZoneName::Hand);
            s.play(&lunar, at_hero("p2")); // the 3rd
            assert_eq!(s.unit(P1, 1).map(|card| card.def_id), Some(WARDRUM.to_string()));
        }

        #[test]
        fn r578_a_play_whose_card_has_since_ceased_to_exist_still_counts_a_frostspatula_that_died_as_a_unit_is_the_1st() {
            let mut s = scenario(json!({
                "p1": { "hand": [WARDRUM, SPATULA, REPLENISH, REPLENISH], "library": LIBRARY, "mana": 10 },
                "p2": { "hand": [FILLER], "field": [TIMMY], "library": LIBRARY },
            }));
            let spatula = s.card(SPATULA).id.clone();
            s.play(&spatula, json!({ "zone": 5 })); // the 1st: it animates into unit zone 5
            s.attack(&spatula, TIMMY); // Tempo Timmy's First Strike kills it first: a token that dies ceases to exist
            s.expect_in_zone(&spatula, "gone");
            play_from_hand(&mut s, REPLENISH, json!({})); // the 2nd
            assert_eq!(wardrum_in(&s), ZoneName::Hand);
            play_from_hand(&mut s, REPLENISH, json!({})); // the 3rd
            assert_eq!(wardrum_in(&s), ZoneName::Field);
        }

        #[test]
        fn the_4th_doesnt_a_zone_that_opens_only_for_the_4th_play_leaves_it_in_hand() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [WARDRUM, REPLENISH, REPLENISH, REPLENISH, LUNAR],
                    "field": [POINTMASTER, MENACE, MENACE, MENACE, MENACE],
                    "library": LIBRARY,
                    "mana": 4,
                },
                "p2": { "hand": [FILLER] },
            }));
            play_three(&mut s);
            assert_eq!(wardrum_in(&s), ZoneName::Hand); // the 3rd found the board full
            let pointmaster = s.card(POINTMASTER).id.clone();
            play_from_hand(&mut s, LUNAR, json!({ "targets": [{ "pick": "instance", "instanceId": pointmaster }] }));
            assert!(s.unit(P1, 1).is_none()); // the 4th opened lane 1
            assert_eq!(wardrum_in(&s), ZoneName::Hand);
        }

        #[test]
        fn from_the_deck_too_summoned_out_of_the_library() {
            let mut s = scenario(json!({
                "p1": { "hand": [LUNAR, LUNAR, LUNAR, FILLER], "library": [FILLER, WARDRUM, FILLER], "mana": 10 },
                "p2": { "hand": [FILLER] },
            }));
            for _ in 0..3 {
                play_from_hand(&mut s, LUNAR, at_hero("p2"));
            }
            assert_eq!(s.unit(P1, 1).map(|card| card.def_id), Some(WARDRUM.to_string()));
            assert_eq!(s.pile(P1, "library").len(), 2);
        }

        #[test]
        fn with_no_open_unit_zone_it_stays_where_it_is() {
            let mut s = scenario(json!({
                "p1": { "hand": [WARDRUM, REPLENISH, REPLENISH, REPLENISH], "field": [MENACE, MENACE, MENACE, MENACE, MENACE], "library": LIBRARY },
                "p2": { "hand": [FILLER] },
            }));
            play_three(&mut s);
            assert_eq!(wardrum_in(&s), ZoneName::Hand);
        }

        #[test]
        fn r97_a_trigger_that_does_not_fire_shows_the_opponent_nothing_the_summon_is_the_first_they_see() {
            let mut s = scenario(json!({
                "p1": { "hand": [WARDRUM, REPLENISH, REPLENISH, REPLENISH], "library": LIBRARY },
                "p2": { "hand": [FILLER] },
            }));
            play_from_hand(&mut s, REPLENISH, json!({}));
            play_from_hand(&mut s, REPLENISH, json!({}));
            let id = s.card(WARDRUM).id.clone();
            assert!(!view_text(&s, P2).contains(WARDRUM));
            let their_events = serde_json::to_string(&view(&s, P2)["events"]).expect("events are JSON");
            assert!(!their_events.contains(&id));
            play_from_hand(&mut s, REPLENISH, json!({}));
            let theirs = view(&s, P2);
            assert_eq!(theirs["opponent"]["units"][0]["defId"], WARDRUM);
            let shown = theirs["events"].as_array().cloned().unwrap_or_default();
            assert!(shown.iter().any(|event| event["type"] == "summoned" && event["defId"] == WARDRUM));
        }

        #[test]
        fn r195_in_hand_it_glows_when_the_next_such_play_would_summon_it_and_the_play_then_does() {
            let mut s = scenario(json!({
                "p1": { "hand": [WARDRUM, REPLENISH, REPLENISH, REPLENISH], "library": LIBRARY },
                "p2": { "hand": [FILLER] },
            }));
            assert!(!glows(&s));
            play_from_hand(&mut s, REPLENISH, json!({}));
            assert!(!glows(&s));
            play_from_hand(&mut s, REPLENISH, json!({}));
            assert!(glows(&s));
            play_from_hand(&mut s, REPLENISH, json!({}));
            assert_eq!(wardrum_in(&s), ZoneName::Field);
        }

        #[test]
        fn r195_it_does_not_glow_when_the_board_is_full_and_the_play_then_summons_nothing() {
            let mut s = scenario(json!({
                "p1": { "hand": [WARDRUM, REPLENISH, REPLENISH, REPLENISH], "field": [MENACE, MENACE, MENACE, MENACE, MENACE], "library": LIBRARY },
                "p2": { "hand": [FILLER] },
            }));
            play_from_hand(&mut s, REPLENISH, json!({}));
            play_from_hand(&mut s, REPLENISH, json!({}));
            assert!(!glows(&s));
            play_from_hand(&mut s, REPLENISH, json!({}));
            assert_eq!(wardrum_in(&s), ZoneName::Hand);
            // The opponent never sees the flag.
            assert!(!view_text(&s, P2).contains("conditionActive"));
        }

        #[test]
        fn r386_the_threshold_reads_through_param_an_upgrade_makes_it_2_and_never_below_2() {
            let mut s = scenario(json!({
                "p1": { "hand": [WARDRUM, REPLENISH, REPLENISH, REPLENISH], "library": LIBRARY },
                "p2": { "hand": [FILLER] },
            }));
            step_param(s.card_mut(WARDRUM), "threshold", -1);
            assert!(!glows(&s));
            play_from_hand(&mut s, REPLENISH, json!({}));
            assert!(glows(&s));
            play_from_hand(&mut s, REPLENISH, json!({}));
            assert_eq!(wardrum_in(&s), ZoneName::Field);

            let mut floor = scenario(json!({ "p1": { "hand": [WARDRUM, REPLENISH, REPLENISH], "library": LIBRARY }, "p2": { "hand": [FILLER] } }));
            step_param(floor.card_mut(WARDRUM), "threshold", -5);
            play_from_hand(&mut floor, REPLENISH, json!({}));
            assert_eq!(wardrum_in(&floor), ZoneName::Hand);
            play_from_hand(&mut floor, REPLENISH, json!({}));
            assert_eq!(wardrum_in(&floor), ZoneName::Field);
        }

        #[test]
        fn end_of_turn_casts_a_copy_of_the_one_spell_played_this_turn_by_definition_and_face() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": BONE_STORM, "radiant": true }, FILLER], "field": [WARDRUM], "library": LIBRARY },
                "p2": { "hand": [FILLER], "field": [MENACE], "library": LIBRARY },
            }));
            s.play(BONE_STORM, json!({})); // Radiant: Echo, 2 hits on each enemy.
            s.expect_stats(MENACE, json!({ "health": 7 }));
            s.end_turn(); // the copy: Radiant too, 2 more hits.
            s.expect_stats(MENACE, json!({ "health": 5 }));
            let copies: Vec<CardInstance> =
                s.pile(P1, "graveyard").into_iter().filter(|card| card.def_id == BONE_STORM).collect();
            assert_eq!(copies.len(), 2);
            assert!(copies.iter().all(|card| card.radiant));
        }

        #[test]
        fn end_of_turn_one_random_card_among_several_each_possible() {
            let mut seen: BTreeSet<String> = BTreeSet::new();
            for i in 0..12 {
                let mut s = scenario(json!({
                    "seed": format!("wd-r{i}"),
                    "p1": { "hand": [BONE_STORM, MANA_WELL, FILLER], "field": [WARDRUM], "library": LIBRARY, "mana": 6 },
                    "p2": { "hand": [FILLER], "library": LIBRARY },
                }));
                s.play(BONE_STORM, json!({}));
                s.play(MANA_WELL, json!({}));
                let from = s.events().len();
                s.end_turn();
                let casts: Vec<Value> = events_json(&s)[from..]
                    .iter()
                    .filter(|event| event["type"] == "cardAnnounced" && event["player"] == "p1")
                    .cloned()
                    .collect();
                assert_eq!(casts.len(), 1);
                if let Some(def_id) = casts[0]["defId"].as_str() {
                    seen.insert(def_id.to_string());
                }
            }
            let mut expected = vec![MANA_WELL.to_string(), BONE_STORM.to_string()];
            expected.sort();
            assert_eq!(seen.into_iter().collect::<Vec<_>>(), expected);
        }

        #[test]
        fn none_played_nothing_a_unit_played_is_not_a_candidate() {
            let mut s = scenario(json!({
                "p1": { "hand": [VANILLA, FILLER], "field": [WARDRUM], "library": LIBRARY },
                "p2": { "hand": [FILLER], "library": LIBRARY },
            }));
            s.play(VANILLA, json!({}));
            let from = s.events().len();
            s.end_turn();
            assert!(!events_json(&s)[from..].iter().any(|event| event["type"] == "cardAnnounced" && event["player"] == "p1"));
        }

        #[test]
        fn r33_r97_a_trap_copy_is_set_face_down_hidden_from_the_opponent() {
            let mut s = scenario(json!({
                "p1": { "hand": [SHEEPISH, FILLER], "field": [WARDRUM], "library": LIBRARY },
                "p2": { "hand": [FILLER], "library": LIBRARY },
            }));
            s.play(SHEEPISH, json!({ "zone": 1 }));
            s.end_turn();
            let traps: Vec<i32> = (1..=5)
                .filter(|lane| s.backrow(P1, *lane).is_some_and(|card| card.def_id == SHEEPISH))
                .collect();
            assert_eq!(traps.len(), 2);
            for lane in &traps {
                assert!(s.backrow(P1, *lane).and_then(|card| card.face_up) != Some(true));
            }
            let theirs = view(&s, P2);
            for lane in &traps {
                assert_eq!(theirs["opponent"]["backrow"][(*lane - 1) as usize]["faceDown"], true);
            }
            assert!(!serde_json::to_string(&theirs).expect("a view is JSON").contains(SHEEPISH));
        }

        #[test]
        fn a_trap_copy_with_no_open_backrow_zone_fizzles_to_the_graveyard() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [SHEEPISH, FILLER],
                    "field": [WARDRUM],
                    "backrow": [MANA_WELL, MANA_WELL, MANA_WELL, MANA_WELL],
                    "library": LIBRARY,
                },
                "p2": { "hand": [FILLER], "library": LIBRARY },
            }));
            s.play(SHEEPISH, json!({ "zone": 5 }));
            s.end_turn();
            let in_graveyard = s.pile(P1, "graveyard").into_iter().filter(|card| card.def_id == SHEEPISH).count();
            assert_eq!(in_graveyard, 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn casts_a_copy_of_each_spell_field_spell_and_trap_played_this_turn_in_play_order() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [BONE_STORM, MANA_WELL, SHEEPISH, VANILLA, FILLER],
                    "field": [{ "def": WARDRUM, "radiant": true }],
                    "library": LIBRARY,
                    "mana": 8,
                },
                "p2": { "hand": [FILLER], "field": [MENACE], "library": LIBRARY },
            }));
            s.play(BONE_STORM, json!({}));
            s.play(MANA_WELL, json!({}));
            s.play(SHEEPISH, json!({}));
            s.play(VANILLA, json!({}));
            let from = s.events().len();
            s.end_turn();
            let casts: Vec<String> = events_json(&s)[from..]
                .iter()
                .filter(|event| event["type"] == "cardAnnounced" && event["player"] == "p1")
                .filter_map(|event| event["defId"].as_str().map(str::to_string))
                .collect();
            assert_eq!(casts, vec![BONE_STORM, MANA_WELL, SHEEPISH]);
            s.expect_stats(MENACE, json!({ "health": 7 }));
        }

        #[test]
        fn a_copy_that_needs_a_target_asks_its_caster_the_paused_sequence_survives_json_and_goes_on() {
            let mut s = scenario(json!({
                "p1": { "hand": [LUNAR, BONE_STORM, FILLER], "field": [{ "def": WARDRUM, "radiant": true }], "library": LIBRARY, "mana": 8 },
                "p2": { "hand": [FILLER], "field": [MENACE], "library": LIBRARY },
            }));
            s.play(LUNAR, at_hero("p2"));
            s.play(BONE_STORM, json!({}));
            s.end_turn();
            let pending = s.state().pending.clone();
            assert_eq!(pending.as_ref().map(|prompt| prompt.player_id), Some(P1));
            let thawed: GameState =
                serde_json::from_str(&serde_json::to_string(s.state()).expect("the state is JSON")).expect("and back");
            assert_eq!(&thawed, s.state());
            let action = Action::new(
                ActionBody::Answer {
                    choice_id: pending.map(|prompt| prompt.id).unwrap_or_default(),
                    selection: vec![Selection::Hero { player: P2 }],
                },
                P1,
                "wardrum-answer",
            );
            let live = reduce(s.state(), &action);
            let frozen = reduce(&thawed, &action);
            assert_eq!(live.error, None);
            assert_eq!(frozen.state, live.state);
            // Lunar Eclipse 3 twice, Bone Storm 1 twice: 8 in all on p2's hero.
            assert_eq!(live.state.players.p2.hero.health, 30 - 8);
        }
    }
}
