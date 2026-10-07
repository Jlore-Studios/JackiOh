//! #85 Unlicensed Experimentation (SPEC §8.4 row 85): Trap, cost 1, Legendary.
//!   Base:    "When the opponent plays a permanent whose type matches one you control: Fuse it onto
//!             a random permanent of yours of that type"
//!   Radiant: "Onto every such permanent" — the cell restates only which of your permanents receive
//!            it, so everything else is the base clause (§8 Conventions), and R77 spells the radiant
//!            case out: "fuses the played permanent onto each matching permanent separately, one
//!            fusion at a time".
//!
//! ARMING vs FIRING (R61, and `traps.ts`'s own rule). `traps.ts`: "`run` returning `[]` is a trap
//! that fired for nothing — it can never mean 'this event was not mine'", so every condition that
//! must leave this trap armed and face-down lives in the `when` predicate, and `run` is reached only
//! once the trap really is firing. R61 divides the two precisely:
//!   - leaves it ARMED: your own play; a Spell; a token, a Recruit, a copy, a Reborn or a Transform
//!     result; a permanent whose type matches nothing you control.
//!   - FIRES it: the opponent playing or casting a permanent from hand whose type matches one you
//!     control — and then "when no legal target of that type remains, the trap fires, is consumed
//!     and does nothing, and the played permanent stays". An Immutable permanent of yours is still
//!     "one you control" (so the trap fires) but is never chosen as the Fuse target (R23), which is
//!     exactly how that last sentence happens. So `when` counts Immutable permanents and `run`
//!     does not.
//!
//! WHICH EVENT (R17, R61, R70, §10.5). §10.5 step 7's `cardResolved`, not step 4's `cardPlayed`.
//! Both name a play — `playSteps.ts` reports a play from hand and `resolve.castCard` a cast, which
//! R70 makes a play "for every rule that counts or reacts to plays" — but R17 puts this trap at step
//! 7, "after a played permanent's Cry", while step 4 is #41 Sheepish's moment, before it ("Sheepish
//! fires on the `summoned`/`cardPlayed` pair emitted at step 4 … Bear Honeypot, Unstable Clone
//! Machine and Unlicensed Experimentation fire on the events step 7 emits", `traps.ts`). Watching
//! `cardPlayed` would fuse the played permanent away before its own Cry ever ran.
//!
//! A summon — Recruit, a copy, a token, Reborn, a Transform result — emits `summoned` and never
//! either of these, so R61's exclusions are the event's own, with one exception this card has to
//! make itself: a TOKEN CARD can be played from a hand (#75's Rush Token card, Combo-Fodder), and
//! R61 says tokens never set this off. Hence the token check in `playedPermanent`.
//!
//! "PLAYED PERMANENTS ONLY" IS `event.permanent` (R61). Step 7 answers the question itself: the flag
//! says whether the card is still in play at the moment it resolved, which is exactly what this trap
//! needs and what a Spell can never be. It also settles the cases a board re-check would have to
//! guess at — a Unit #41 Sheepish has already transformed away, a token that ceased to exist, R138's
//! cast permanent that found no zone and went to its graveyard — all report false, so none of them
//! arms this trap and nothing here reads a zone.
//!
//! FUSE, VIA THE EFFECTS BARREL (§6.3 Fuse, R77, R23, R61).
//! `fuseCards({ instanceIds, targetInstanceIds, pick })` is the verb. The loop over the targets is
//! inside it rather than here, and that is load-bearing: `subsystems/fuse.ts` has an ingredient
//! cease to exist the moment a fusion is made (`removeFromAnyZone`, then `{ z: "gone" }`), so after
//! the first fusion the played permanent is held by no pile and `findInstance` cannot reach it. An
//! ingredient only ever contributes its DEFINITION, which is why the subsystem says a ceased-to-
//! exist ingredient still fuses — but only something holding the resolved instance can honour that.
//! So this card names its targets once and the verb keeps the ingredient across the fusions.
//! The base face's "a random permanent of yours of that type" is `pick: "random"`, drawn with
//! `ctx.rng` inside the effect so the draw stays in the reducer (§9.3, §10.7).
//! and nothing else about this file changes.
//!
//! The rest is deliberately NOT here: `fireTrap` emits `trapFired`, runs the state check and
//! consumes the trap (a Trap goes to its owner's graveyard, §3.2), and R33 keeps a face-down trap's
//! identity in `viewFor`. R13 leaves a card dormant under a Stack off the field, so it is neither a
//! match nor a target; `cardAt` reads the acting card per zone, which is that rule.
//!
//! THE GLOW (R662). The trap lights up on its controller's field while they control a permanent,
//! other than this trap, that a played permanent could be fused onto (`fusablePermanentsOf`: not
//! Immutable, R23). Which type the opponent will play is theirs to choose, so the glow says the trap
//! has somewhere to land, the same on both faces; R61 still fires it on an Immutable match alone.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-085";

// §10.5 step 7's event: a play or a cast that has finished resolving (R17, R70). TS names it
// `ResolvedEvent = Extract<GameEvent, { type: "cardResolved" }>`; here it is `GameEvent::CardResolved`,
// matched where it is read.

/// R61 and §5.1: "Field Trap counts as Trap", in both directions, so both read as one key.
fn type_key(type_: CardType) -> CardType {
    if type_ == CardType::FieldTrap {
        CardType::Trap
    } else {
        type_
    }
}

/// The permanent this play put on the opponent's field, or null when the event is not one this trap
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
    // R61: "played permanents only". Step 7 read this as it landed, so no board check is needed and
    // a Spell, or a Unit an earlier trap has already taken off the field, is out by the same test.
    if !*permanent {
        return None;
    }

    let card = find_instance(&ctx.state, instance_id)?;

    let played = def_of(Some(&*ctx.state), &card.def_id);
    // R61: "tokens … never set it off", including a token card played from a hand.
    if played.token || played.tags.contains(&Tag::Token) {
        return None;
    }
    Some(card.clone())
}

/// "a permanent of yours of that type" (§8), which R61 narrows twice: the firing trap is "neither
/// matched nor fused onto", and R13 leaves a card dormant under a Stack off the field.
///
/// And never the played card itself. "Fuse IT onto a permanent of yours" names two different cards,
/// yet the played one can already stand on this trap's side when the trap fires: #52 Silly Silas
/// played into its controller's lane 5 and rotated right crosses to this side (§3.1), so a scan of
/// "yours" meets him. Counted, he made "one you control" true for a trap with nothing else to fuse
/// onto, and picked, he was fused onto himself, which `fuse` refuses — the trap was spent for
/// nothing although another permanent of the type was there (R61 spends it only with no legal
/// target). Left out of both, the trap stays armed when he is the only match (R99).
fn matching_permanents(ctx: &EffectContext<'_>, type_: CardType, played: &CardInstance) -> Vec<CardInstance> {
    let wanted = type_key(type_);
    let self_id = ctx.self_.as_ref().map(|card| card.id.clone());
    [Row::Units, Row::Backrow]
        .into_iter()
        .flat_map(|row| {
            slots_of(ctx.controller, row)
                .into_iter()
                .filter_map(|slot| {
                    let card = card_at(&ctx.state, &slot)?;
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
                .filter(|card| !unit_has(&ctx.state, card, KeywordKind::Immutable))
                .map(|card| card.id)
                .collect();

            // R61: with no legal target the trap fires, is consumed and does nothing, and the played
            // permanent stays — which is an empty effect list, and `fireTrap` does the rest.
            if target_ids.is_empty() {
                return vec![];
            }

            // R77: one fusion per target, each its own transient definition, the played permanent
            // contributing its definition to every one of them. `fuseCards` holds the resolved
            // ingredient across the loop, which is why `targetInstanceIds` is one effect and not one
            // effect per target — after the first fusion the played card is in `{ z: "gone" }` and no id
            // can reach it again. The random pick of the base face is `ctx.rng` INSIDE that effect.
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

/// R662: armed while a permanent of its controller's, other than this trap, could take a Fuse.
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

// #85 Unlicensed Experimentation — SPEC §8.4 row 85 ("When the opponent plays a permanent whose
// type matches one you control: Fuse it onto a random permanent of yours of that type" / radiant
// "Onto every such permanent"), BUILD M4-T4 row 85.
//
// Every test puts the trap face-down in p1's backrow lane 3 and makes p2 the active player: a Trap
// answers the OPPONENT's action and resolves to completion before that action continues (§10.3).
//
// The two halves of the card, and the ruling that splits them:
//
//   R99  A condition that must leave the trap ARMED belongs in the trigger's `when`, never in `run`
//        — R61 makes an empty effect list from `run` mean "fired, consumed, did nothing". So the
//        "arming" tests below are the real test of R99: each one is an event this trap must ignore,
//        and after it the trap is still on the field and still face-down.
//   R61  What fires it: the opponent playing or casting a permanent from hand whose type matches
//        one they control. Field Trap counts as Trap; the firing trap is neither matched nor fused
//        onto; an Immutable permanent of yours IS "one you control" (so the trap fires) but is
//        never the Fuse target (R23) — which is the "fires, is consumed, does nothing, and the
//        played permanent stays" case.
//   R17  It fires AFTER the played permanent's Cry, unlike #41 Sheepish, which fires before it.
//   R77/R102  What the Fuse composes: summed stats, united keywords, cost capped at FUSE_COST_CAP
//        4, the target instance kept with its damage and position, the other ingredient ceasing to
//        exist (R86's `{ z: "gone" }`) with no Death trigger and no destroyed counter.
//
// The ingredients are placed by `field`/played from `hand` and chosen so no Cry muddies the board:
//   #11 Tempo Timmy   1, 3/3, Rush + First Strike, no Cry
//   #25 4-mana 7/7    4, 7/7, Armor 7, no Cry
//   #15 Me and Mr Tok 1, 1/1, Cry: summon a Rush Token — the R17 timing probe
//   #8  Mr. Vanilla   1, 3/3, Immutable, no Cry
//
// R662's yellow glow (`conditionMet`): on its controller's field while they control a permanent other
// than the trap that is not Immutable, both faces, checked against the opponent's play, at the end.
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

    /// An engine value as the JSON the TS test compares it with.
    fn js<T: serde::Serialize>(v: &T) -> Value {
        serde_json::to_value(v).expect("serialises")
    }

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

    /// TS `s.events.findIndex(match)`: -1 when no event matches.
    fn index_of_event(s: &Scenario, matching: impl Fn(&GameEvent) -> bool) -> isize {
        s.events()
            .iter()
            .position(|event| matching(event))
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

        /// TS `R662 ${face}: with a Unit of its controller's it glows for them only, and the opponent's
        /// Unit is fused onto it`.
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

        /// TS `R662 ${face}: alone on its side it does not glow, and the opponent's Unit stays theirs`.
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

        /// TS `R662 ${face}: an Immutable permanent alone takes no Fuse (R23), so it does not glow`.
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
