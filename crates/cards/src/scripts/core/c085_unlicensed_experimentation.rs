//! #85 Unlicensed Experimentation (SPEC §8.4 row 85): Trap, cost 1, Legendary.
//!   Base:    "When the opponent plays a permanent whose type matches one you control: Fuse it onto
//!             a random permanent of yours of that type"
//!   Radiant: "Onto every such permanent": only the targets change (§8 Conventions); R77 fuses the
//!            played permanent onto each matching permanent separately, one fusion at a time.
//!
//! ARMING vs FIRING (R61): what must leave the trap armed and face-down (your own play, a Spell, a
//! token, a Recruit, copy, Reborn or Transform result, a type matching nothing of yours) lives in
//! `when`, never in `run`, whose empty list means "fired for nothing". An Immutable permanent of
//! yours still counts in `when` ("one you control") but `run` never picks it (R23).
//! EVENT: `CardResolved` (§10.5 step 7, R17, R70), not `CardPlayed` (step 4, #41 Sheepish's moment):
//! the trap fires after the played permanent's Cry, and `event.permanent` says it is still in play.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-085";

// §10.5 step 7's event: a play or a cast that has finished resolving (R17, R70), `GameEvent::CardResolved`.

/// R61 and §5.1: "Field Trap counts as Trap", in both directions, so both read as one key.
fn type_key(type_: CardType) -> CardType {
    if type_ == CardType::FieldTrap {
        CardType::Trap
    } else {
        type_
    }
}

/// The permanent this play put on the opponent's field, or `None` when the event is not one this trap
/// answers: the controller's own play, a Spell or anything else that is no longer in play (R61's
/// `permanent`), a token (R61), or a card the instance table can no longer name.
fn played_permanent(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<CardInstance> {
    let GameEvent::CardResolved {
        player,
        instance_id,
        permanent,
        ..
    } = event
    else {
        return None;
    };
    // §8: "the opponent plays". A trap never answers its own controller's play.
    if *player == ctx.controller {
        return None;
    }
    // R61: "played permanents only". Step 7 read this as it landed, so no board check is needed: a
    // Spell, a Unit an earlier trap took off the field and R138's cast permanent that found no zone
    // are out by the same test.
    if !*permanent {
        return None;
    }

    let card = find_instance(ctx.state, instance_id)?;

    let played = def_of(Some(&*ctx.state), &card.def_id);
    // R61: "tokens … never set it off", including a token card played from a hand.
    if played.token || played.tags.contains(&Tag::Token) {
        return None;
    }
    Some(card.clone())
}

/// "a permanent of yours of that type" (§8), which R61 narrows twice: the firing trap is "neither
/// matched nor fused onto", and R13 leaves a card dormant under a Stack off the field. Never the
/// played card either: #52 Silly Silas rotated onto this side (§3.1) would be counted, then fused onto
/// himself, which `fuse` refuses, spending the trap for nothing although another permanent of the
/// type was there (R61 spends it only with no legal target). Left out of both, the trap stays armed when he is the only match (R99).
fn matching_permanents(ctx: &EffectContext<'_>, type_: CardType, played: &CardInstance) -> Vec<CardInstance> {
    let wanted = type_key(type_);
    let self_id = ctx.self_.as_ref().map(|card| card.id.clone());
    [Row::Units, Row::Backrow]
        .into_iter()
        .flat_map(|row| {
            slots_of(ctx.controller, row)
                .into_iter()
                .filter_map(|slot| {
                    let card = card_at(ctx.state, slot)?;
                    if Some(&card.id) == self_id.as_ref() || card.id == played.id {
                        return None;
                    }
                    if type_key(def_of(Some(&*ctx.state), &card.def_id).type_) == wanted {
                        Some(card.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<CardInstance>>()
        })
        .collect()
}

/// `on_all` is the radiant face: R77's "onto each matching permanent separately".
fn experimentation(on_all: bool) -> TrapTrigger {
    TriggerDef::new(
        if on_all {
            "85r-unlicensed-experimentation"
        } else {
            "85-unlicensed-experimentation"
        },
        &[GameEventType::CardResolved],
        move |ctx, event| {
            if !matches!(event, GameEvent::CardResolved { .. }) {
                return vec![];
            }
            let Some(played) = played_permanent(ctx, event) else {
                return vec![];
            };

            // R23: "Immutable permanents are never chosen" as the Fuse target.
            let type_ = def_of(Some(&*ctx.state), &played.def_id).type_;
            let target_ids: Vec<String> = matching_permanents(ctx, type_, &played)
                .into_iter()
                .filter(|card| !unit_has(ctx.state, card, KeywordKind::Immutable))
                .map(|card| card.id)
                .collect();

            // R61: with no legal target the trap fires, is consumed (a Trap goes to its owner's
            // graveyard, §3.2) and does nothing, and the played permanent stays: an empty effect list.
            if target_ids.is_empty() {
                return vec![];
            }

            // R77, §6.3: one fusion per target, each its own transient definition. One effect names all
            // the targets because `fuse_cards` holds the resolved ingredient across them (after the first
            // fusion the played card is gone and no id reaches it); the base face's random pick is
            // `ctx.rng` inside that effect, so the draw stays in the reducer (§9.3, §10.7).
            let pick = if on_all { "all" } else { "random" };
            vec![fuse_cards(json_as(json!({
                "instanceIds": [played.id],
                "targetInstanceIds": target_ids,
                "pick": pick,
            })))]
        },
    )
    .with_when(|ctx, event| {
        if !matches!(event, GameEvent::CardResolved { .. }) {
            return false;
        }
        let Some(played) = played_permanent(ctx, event) else {
            return false;
        };
        // §8: "whose type matches one you control". R61 counts an Immutable permanent of yours here,
        // so the trap fires and is consumed even though nothing can be fused onto it.
        let type_ = def_of(Some(&*ctx.state), &played.def_id).type_;
        !matching_permanents(ctx, type_, &played).is_empty()
    })
}

/// R662: armed while a permanent of its controller's, other than this trap, could take a Fuse (not
/// Immutable, R23). Which type the opponent plays is theirs to choose, so the glow only says the Fuse
/// has somewhere to land, the same on both faces; R61 still fires the trap on an Immutable match alone.
fn condition_met() -> ConditionHook {
    condition_hook(|ctx| {
        ctx.zone == ConditionZone::Field
            && !fusable_permanents_of(ctx.state, ctx.controller, Some(ctx.self_.id.as_str())).is_empty()
    })
}

pub fn script() -> CardScripts {
    let condition_met = condition_met();
    let base = Script {
        triggers: vec![experimentation(false)],
        condition_met: Some(condition_met.clone()),
        ..Script::default()
    };
    // "Onto every such permanent" (R77: one fusion at a time, each target keeping its own instance).
    let radiant = Script {
        triggers: vec![experimentation(true)],
        condition_met: Some(condition_met),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// #85 Unlicensed Experimentation — SPEC §8.4 row 85, BUILD M4-T4 row 85. Every test puts the trap
// face-down in p1's backrow lane 3 with p2 active: a Trap answers the OPPONENT's action and resolves
// to completion before that action continues (§10.3). The "arming" tests are R99's proof: an event
// the trap must ignore leaves it on the field, face-down. R61 and R23 say what fires it (an Immutable
// match fires it and takes no Fuse), R17 times it after the Cry, R77/R102/R86 compose the Fuse (the
// other ingredient ceases to exist, with no Death trigger and no destroyed counter).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TRAP: &str = "core-085"; // Trap, 1, Legendary
    const TIMMY: &str = "core-011"; // Unit, 1 — 3/3 Rush, First Strike
    const BIG_UNIT: &str = "core-025"; // Unit, 4 — 7/7 Armor 7
    const MR_TOKEN: &str = "core-015"; // Unit, 1 — 1/1, Cry: summon a Rush Token
    const MR_VANILLA: &str = "core-008"; // Unit, 1 — 4/4, no text

    /// A Radiant #19 Midrange Menace: an 18/18 with Taunt and Immutable (§8.1 row 19).
    fn immutable() -> Value {
        json!({ "def": "core-019", "radiant": true })
    }

    const RUSH_TOKEN: &str = "core-t-rush"; // the unit-token CARD, playable from a hand (§5, #75)
    const STOCKPILE: &str = "core-005"; // Spell, 1
    const CALL_TO_ARMS: &str = "core-069"; // Spell, 2 — "Recruit 3 Units costing 1 or less"
    const MY_PAWN: &str = "core-096"; // Trap, 1
    const INTERN: &str = "core-071"; // Field Trap, 1
    const MENACE: &str = "core-019";

    use crate::js;

    /// The trap as it sits in play: face-down in lane 3, so lanes 1 and 2 are free for the board.
    fn armed(radiant: bool) -> Value {
        let mut entry = json!({ "def": TRAP, "lane": 3 });
        if radiant {
            entry["radiant"] = json!(true);
        }
        entry
    }

    fn count_of(s: &Scenario, type_: GameEventType) -> usize {
        s.events().iter().filter(|event| event.event_type() == type_).count()
    }

    /// A trap that declined the event is still there and still hidden (§5.1, R33).
    fn expect_still_armed(s: &Scenario) {
        assert_eq!(count_of(s, GameEventType::TrapFired), 0);
        let trap = s.backrow(P1, 3);
        assert!(trap.is_some());
        let trap = trap.unwrap();
        assert_eq!(trap.def_id, TRAP);
        assert_ne!(trap.face_up, Some(true));
    }

    fn hand_of(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.pile(player, "hand")
    }

    fn first_of(s: &Scenario, player: PlayerId, def_id: &str) -> CardInstance {
        hand_of(s, player)
            .into_iter()
            .find(|held| held.def_id == def_id)
            .unwrap_or_else(|| panic!("{player} has no {def_id} in hand"))
    }

    /// The index of the first matching event, -1 when none matches.
    fn index_of_event(s: &Scenario, matching: impl Fn(&GameEvent) -> bool) -> isize {
        s.events()
            .iter()
            .position(matching)
            .map(|index| index as isize)
            .unwrap_or(-1)
    }

    mod n85_unlicensed_experimentation_r99_the_events_that_leave_it_armed {
        use super::*;

        #[test]
        fn r99_the_opponent_plays_its_own_controller_s_play_never_sets_it_off() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "backrow": [armed(false)], "field": [TIMMY], "hand": [BIG_UNIT, STOCKPILE] },
                "p2": { "hand": [STOCKPILE, MENACE] },
            }));

            s.play(BIG_UNIT, json!({ "zone": 2 }));

            expect_still_armed(&s);
            s.expect_in_zone(BIG_UNIT, "field");
        }

        #[test]
        fn r99_a_spell_is_not_a_permanent_so_the_opponent_casting_one_leaves_it_armed() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)], "field": [TIMMY] },
                "p2": { "hand": [STOCKPILE, MENACE], "library": [MENACE, TIMMY, MENACE] },
            }));

            s.play(STOCKPILE, json!({}));

            expect_still_armed(&s);
        }

        #[test]
        fn r99_a_permanent_whose_type_matches_nothing_you_control_leaves_it_armed() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)] }, // no Unit of p1's anywhere
                "p2": { "hand": [BIG_UNIT, STOCKPILE] },
            }));

            s.play(BIG_UNIT, json!({ "zone": 1 }));

            expect_still_armed(&s);
            s.expect_in_zone(BIG_UNIT, "field");
        }

        #[test]
        fn r61_the_firing_trap_is_never_its_own_match_a_trap_play_with_no_other_trap_of_yours_leaves_it_armed() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)] }, // the only Trap p1 controls is #85 itself
                "p2": { "hand": [MY_PAWN, STOCKPILE] },
            }));

            s.play(MY_PAWN, json!({ "zone": 1 }));

            expect_still_armed(&s);
            s.expect_in_zone(MY_PAWN, "field");
        }

        #[test]
        fn r61_a_token_card_the_opponent_plays_never_sets_it_off() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)], "field": [TIMMY] },
                "p2": { "hand": [RUSH_TOKEN, STOCKPILE] },
            }));

            s.play(RUSH_TOKEN, json!({ "zone": 1 }));

            expect_still_armed(&s);
        }

        #[test]
        fn r61_recruit_never_sets_it_off_the_units_arrive_by_summoned_and_the_spell_is_no_permanent() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)], "field": [TIMMY] },
                "p2": { "hand": [CALL_TO_ARMS, STOCKPILE], "library": [TIMMY, MR_VANILLA, TIMMY] },
            }));

            s.play(CALL_TO_ARMS, json!({}));

            // The Recruit really happened — units reached p2's board — and the trap still did not fire.
            assert!(count_of(&s, GameEventType::Summoned) > 0);
            expect_still_armed(&s);
        }
    }

    mod n85_unlicensed_experimentation_base_when_it_fires {
        use super::*;

        #[test]
        fn s8_4_fires_on_the_opponent_s_matching_permanent_and_fuses_it_onto_yours_r77() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)], "field": [{ "def": TIMMY, "damage": 1, "position": "DEF" }] },
                "p2": { "hand": [BIG_UNIT, STOCKPILE] },
            }));
            let trap = s.backrow(P1, 3).expect("the trap");
            let mine = s.unit(P1, 1).expect("p1's Timmy");
            let played = first_of(&s, P2, BIG_UNIT);

            s.play(BIG_UNIT, json!({ "zone": 1 }));

            s.expect_events(json!(["cardPlayed", "trapFired", "fused"]));
            s.expect_in_zone(&trap, "graveyard");
            // R77: the target instance is kept, with its damage and its position.
            s.expect_stats(&mine, json!({ "attack": 10, "maxHealth": 10, "health": 9 }));
            assert_eq!(s.card(&mine).id, mine.id);
            assert_eq!(s.card(&mine).position, Some(Position::Def));
            // R102: the capped sum of the printed costs, min(1 + 4, 4).
            let fused_def = s.card(&mine).def_id.clone();
            assert_eq!(
                s.state().transient_defs.get(&fused_def).map(|def| js(&def.cost)),
                Some(json!(4))
            );
            // R102/R86: the consumed ingredient ceases to exist — no graveyard, no Death, no counter.
            s.expect_in_zone(&played, "gone");
            assert!(s.pile(P2, "graveyard").is_empty());
            assert_eq!(count_of(&s, GameEventType::Destroyed), 0);
            assert_eq!(s.state().counters.destroyed, 0);
        }

        #[test]
        fn r77_the_fused_keywords_are_the_union_of_the_ingredients() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)], "field": [TIMMY] },
                "p2": { "hand": [BIG_UNIT, STOCKPILE] },
            }));
            let mine = s.unit(P1, 1).expect("p1's Timmy");

            s.play(BIG_UNIT, json!({ "zone": 1 }));

            let kinds: Vec<KeywordKind> = s.stats(&mine).keywords.iter().map(|keyword| keyword.kind()).collect();
            for kind in [KeywordKind::Rush, KeywordKind::FirstStrike, KeywordKind::Armor] {
                assert!(kinds.contains(&kind));
            }
            assert_eq!(s.stats(&mine).armor, 7); // #25's Armor 7 survived the union
        }

        #[test]
        fn s5_1_a_trap_is_consumed_when_it_fires_and_goes_to_its_owner_s_graveyard() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)], "field": [TIMMY] },
                "p2": { "hand": [BIG_UNIT, STOCKPILE] },
            }));
            let trap = s.backrow(P1, 3).expect("the trap");

            s.play(BIG_UNIT, json!({ "zone": 1 }));

            s.expect_in_zone(&trap, "graveyard");
            assert_eq!(count_of(&s, GameEventType::TrapFired), 1);
            assert!(s.backrow(P1, 3).is_none());
        }

        #[test]
        fn s8_4_a_random_permanent_of_yours_the_base_face_fuses_onto_exactly_one_of_two_matches() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-085-random-one",
                "active": "p2",
                "p1": { "backrow": [armed(false)], "field": [TIMMY, MR_TOKEN] },
                "p2": { "hand": [BIG_UNIT, STOCKPILE] },
            }));

            s.play(BIG_UNIT, json!({ "zone": 1 }));

            assert_eq!(count_of(&s, GameEventType::Fused), 1);
            let first = s.unit(P1, 1).expect("lane 1");
            let second = s.unit(P1, 2).expect("lane 2");
            let timmy_grew = s.stats(&first).attack == 10;
            let token_grew = s.stats(&second).attack == 8;
            assert_eq!([timmy_grew, token_grew].into_iter().filter(|grew| *grew).count(), 1);
        }

        #[test]
        fn r61_field_trap_counts_as_trap_and_the_result_is_promoted_to_field_trap_r77() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                // My Pawn watches `attackDeclared`, so it is a Trap of p1's that this play cannot wake.
                "p1": { "backrow": [{ "def": MY_PAWN, "lane": 1 }, armed(false)] },
                "p2": { "hand": [INTERN, STOCKPILE] },
            }));
            let pawn = s.backrow(P1, 1).expect("My Pawn");
            let played = first_of(&s, P2, INTERN);

            s.play(INTERN, json!({ "zone": 1 }));

            s.expect_events(json!(["cardPlayed", "trapFired", "fused"]));
            let fused_def = s.card(&pawn).def_id.clone();
            assert_eq!(
                s.state().transient_defs.get(&fused_def).map(|def| def.type_),
                Some(CardType::FieldTrap)
            );
            s.expect_in_zone(&played, "gone");
        }

        #[test]
        fn r23_r61_an_immutable_permanent_of_yours_fires_the_trap_and_receives_nothing_and_the_played_card_stays() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)], "field": [immutable()] },
                "p2": { "hand": [BIG_UNIT, STOCKPILE] },
            }));
            let trap = s.backrow(P1, 3).expect("the trap");
            let vanilla = s.unit(P1, 1).expect("the Menace");
            let played = first_of(&s, P2, BIG_UNIT);

            s.play(BIG_UNIT, json!({ "zone": 1 }));

            // It fired and was consumed — an Immutable permanent still counts as "one you control" (R61).
            s.expect_events(json!(["cardPlayed", "trapFired"]));
            assert_eq!(count_of(&s, GameEventType::TrapFired), 1);
            s.expect_in_zone(&trap, "graveyard");
            // …and did nothing: no fusion, the Immutable body untouched, the played permanent still there.
            assert_eq!(count_of(&s, GameEventType::Fused), 0);
            s.expect_stats(&vanilla, json!({ "attack": 18, "maxHealth": 18 }));
            s.expect_in_zone(&played, "field");
        }

        #[test]
        fn r17_it_fires_after_the_played_permanent_s_cry_so_the_cry_has_already_resolved() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)], "field": [TIMMY] },
                "p2": { "hand": [MR_TOKEN, STOCKPILE] },
            }));

            s.play(MR_TOKEN, json!({ "zone": 1 }));

            // #15's Cry summons a Rush Token for p2. R17 puts this trap at §10.5 step 7, after that.
            let cry = index_of_event(
                &s,
                |event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == RUSH_TOKEN),
            );
            let fired = index_of_event(&s, |event| matches!(event, GameEvent::TrapFired { .. }));
            assert!(cry >= 0);
            assert!(fired >= 0);
            assert!(fired > cry);
        }
    }

    mod n85_unlicensed_experimentation_radiant {
        use super::*;

        #[test]
        fn r77_onto_every_such_permanent_one_fusion_at_a_time_each_target_keeping_its_own_instance() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(true)], "field": [TIMMY, MR_TOKEN] },
                "p2": { "hand": [BIG_UNIT, STOCKPILE] },
            }));
            let first = s.unit(P1, 1).expect("lane 1");
            let second = s.unit(P1, 2).expect("lane 2");
            let played = first_of(&s, P2, BIG_UNIT);

            s.play(BIG_UNIT, json!({ "zone": 1 }));

            assert_eq!(count_of(&s, GameEventType::TrapFired), 1);
            assert_eq!(count_of(&s, GameEventType::Fused), 2);
            s.expect_stats(&first, json!({ "attack": 10, "maxHealth": 10 })); // 3/3 + 7/7
            s.expect_stats(&second, json!({ "attack": 8, "maxHealth": 8 })); // 1/1 + 7/7
            assert_eq!(s.card(&first).id, first.id);
            assert_eq!(s.card(&second).id, second.id);
            s.expect_in_zone(&played, "gone");
        }

        #[test]
        fn s8_conventions_the_radiant_cell_restates_only_the_targets_so_a_single_match_still_works() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(true)], "field": [TIMMY] },
                "p2": { "hand": [BIG_UNIT, STOCKPILE] },
            }));
            let mine = s.unit(P1, 1).expect("p1's Timmy");

            s.play(BIG_UNIT, json!({ "zone": 1 }));

            assert_eq!(count_of(&s, GameEventType::Fused), 1);
            s.expect_stats(&mine, json!({ "attack": 10, "maxHealth": 10 }));
        }

        #[test]
        fn r99_the_radiant_face_declines_the_same_events_its_controller_s_own_play_leaves_it_armed() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "backrow": [armed(true)], "field": [TIMMY], "hand": [BIG_UNIT, STOCKPILE] },
                "p2": { "hand": [STOCKPILE, MENACE] },
            }));

            s.play(BIG_UNIT, json!({ "zone": 2 }));

            expect_still_armed(&s);
        }

        #[test]
        fn r23_the_radiant_face_skips_an_immutable_permanent_and_fuses_onto_the_rest() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(true)], "field": [immutable(), TIMMY] },
                "p2": { "hand": [BIG_UNIT, STOCKPILE] },
            }));
            let vanilla = s.unit(P1, 1).expect("the Menace");
            let timmy = s.unit(P1, 2).expect("Timmy");

            s.play(BIG_UNIT, json!({ "zone": 1 }));

            assert_eq!(count_of(&s, GameEventType::Fused), 1);
            s.expect_stats(&vanilla, json!({ "attack": 18, "maxHealth": 18 }));
            s.expect_stats(&timmy, json!({ "attack": 10, "maxHealth": 10 }));
        }
    }

    mod n85_unlicensed_experimentation_glows_while_a_fuse_has_somewhere_to_land_r662 {
        use super::*;

        const VANILLA: &str = "core-008"; // Mr. Vanilla, a 1-cost Unit
        const MENACE: &str = "core-019"; // Midrange Menace; its Radiant face is Immutable

        fn face_of(radiant: bool) -> &'static str {
            if radiant {
                "radiant"
            } else {
                "base"
            }
        }

        fn with_a_unit_it_glows_and_fuses(radiant: bool) {
            let face = face_of(radiant);
            let mut s = scenario(json!({
                "seed": format!("r662-085-{face}-on"),
                "active": "p2",
                "p1": { "backrow": [{ "def": "core-085", "radiant": radiant }], "field": [VANILLA] },
                "p2": { "hand": [VANILLA, "core-010"] },
            }));
            assert!(backrow_glows(&s, 1, P1));
            assert!(!opponent_sees_glow(&s, 1, P1));

            s.play(VANILLA, json!({}));
            assert!(s.events().iter().any(|event| matches!(event, GameEvent::TrapFired { .. })));
            assert!(s.unit(P2, 1).is_none());
        }

        fn alone_it_does_not_glow(radiant: bool) {
            let face = face_of(radiant);
            let mut s = scenario(json!({
                "seed": format!("r662-085-{face}-off"),
                "active": "p2",
                "p1": { "backrow": [{ "def": "core-085", "radiant": radiant }] },
                "p2": { "hand": [VANILLA, "core-010"] },
            }));
            assert!(!backrow_glows(&s, 1, P1));

            s.play(VANILLA, json!({}));
            assert!(!s.events().iter().any(|event| matches!(event, GameEvent::TrapFired { .. })));
            assert_eq!(s.unit(P2, 1).map(|card| card.def_id.clone()), Some(VANILLA.to_string()));
        }

        fn an_immutable_permanent_alone_does_not_glow(radiant: bool) {
            let face = face_of(radiant);
            let mut s = scenario(json!({
                "seed": format!("r662-085-{face}-immutable"),
                "active": "p2",
                "p1": {
                    "backrow": [{ "def": "core-085", "radiant": radiant }],
                    "field": [{ "def": MENACE, "radiant": true }],
                },
                "p2": { "hand": [VANILLA, "core-010"] },
            }));
            assert!(!backrow_glows(&s, 1, P1));

            // R61: it still fires and is consumed, and the Unit stays where it landed.
            s.play(VANILLA, json!({}));
            assert_eq!(s.unit(P2, 1).map(|card| card.def_id.clone()), Some(VANILLA.to_string()));
        }

        #[test]
        fn r662_base_with_a_unit_of_its_controller_s_it_glows_for_them_only_and_the_opponent_s_unit_is_fused_onto_it() {
            crate::register_all();
            with_a_unit_it_glows_and_fuses(false);
        }

        #[test]
        fn r662_base_alone_on_its_side_it_does_not_glow_and_the_opponent_s_unit_stays_theirs() {
            crate::register_all();
            alone_it_does_not_glow(false);
        }

        #[test]
        fn r662_base_an_immutable_permanent_alone_takes_no_fuse_r23_so_it_does_not_glow() {
            crate::register_all();
            an_immutable_permanent_alone_does_not_glow(false);
        }

        #[test]
        fn r662_radiant_with_a_unit_of_its_controller_s_it_glows_for_them_only_and_the_opponent_s_unit_is_fused_onto_it() {
            crate::register_all();
            with_a_unit_it_glows_and_fuses(true);
        }

        #[test]
        fn r662_radiant_alone_on_its_side_it_does_not_glow_and_the_opponent_s_unit_stays_theirs() {
            crate::register_all();
            alone_it_does_not_glow(true);
        }

        #[test]
        fn r662_radiant_an_immutable_permanent_alone_takes_no_fuse_r23_so_it_does_not_glow() {
            crate::register_all();
            an_immutable_permanent_alone_does_not_glow(true);
        }
    }
}
