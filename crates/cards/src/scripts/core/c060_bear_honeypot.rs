//! #60 Bear Honeypot (SPEC §8.3): Trap, cost 1, Epic. "When your opponent plays a (1) Cost or less
//! card, if you have an empty unit zone: Summon 2 Rush Tokens. If it's a Unit, they attack it." /
//! radiant "When your opponent plays a card, if you have an empty unit zone: Fill your board with Rush
//! Tokens. If it's a Unit, they attack it." Engine cell: "Fires after the played card resolves
//! (ruling), so the unit is on the field; forced attacks per 4.2 during the opponent's turn; cost =
//! cost paid".
//!
//! The Radiant face drops the cost threshold, fills the board in place of the two tokens, and keeps
//! the rest — the empty-zone condition and the attack on a played Unit (§8 Conventions, R277).
//!
//! R430 (patch v0.2.0): "if you have an empty unit zone" is part of the condition. While its
//! controller has no empty, unlocked, unreserved unit zone — R64's zones a summon may take, the same
//! `open_zones` a summon places into — the trap does not fire and is not consumed: it stays face-down
//! and armed for a play that comes when there is room. So it can never be spent on a board that
//! cannot take a single token.
//!
//! What this card does NOT do, because §5.1, §10.3 and traps.rs own it: emit `trapFired`, run the
//! post-trap state check, or send itself to the graveyard. `fire_trap` does all three, and because
//! this is a Trap and not a Field Trap it is consumed by `consume_trap` the moment it fires — once
//! per game, whatever the effects achieved (R17, R61).
//!
//! THE CONDITION IS A `when`, NOT AN EARLY RETURN FROM `run`. traps.ts: "`run` returning `[]` is a
//! trap that fired for nothing — it can never mean 'this event was not mine'. A condition that must
//! leave the trap armed (Bear Honeypot's 'costing 1 or less' …) therefore belongs in a predicate".
//! A 2-cost play must leave this trap face-down and armed, so the threshold lives in `when`.
//!
//! R56: "'Costing 1 or less' … uses the cost actually paid after modifiers", which is
//! `event.costPaid`, not the printed cost. R70: "a cast is free and counts as a play … with cost
//! paid 0", and names this card, so a cast card is always "costing 1 or less".
//!
//! R53 (all of it) is the forced attack: skip §4.2 steps 1-3, so position, summoning sickness and
//! Taunt are ignored and no exertion is spent; the target still strikes back; the attackers go in
//! lane order; each attack is its own combat followed by its own state check; and the next attacker
//! attacks only if the target is still on the field. `force_attacks_on` (engine/src/combat.rs) is
//! exactly that, and `forced_attacks` in the effects library is its Effect wrapper.
//!
//! WHICH EVENT, AND WHY NOT `cardPlayed` (R17, §10.5 steps 4 and 7). This card fires at step 7,
//! "after the card resolves", which is `cardResolved` (`echo::land_after_resolution` emits it once per
//! play or cast, after step 6 has drained every Echo repeat) — the moment #41 Sheepish shares since
//! patch v0.2.0 (R427), the two answering in lane order (R68). Watching `cardPlayed`
//! would be wrong and not merely early: the Engine cell requires the played unit to be on the field
//! with its Cry already resolved, which is what makes "they attack it" reach anything.
//!
//! `cardResolved` also carries `costPaid` for R89's sake — a trigger answering an event reads what
//! it needs OFF the event, because step 7's instance may have been reset since step 4 — so the R56
//! threshold below never re-derives a cost from the board.
//!
//! `summonedThisScript` on the forced-attack filter is what makes "they attack it" mean the tokens
//! THIS trap just summoned rather than every Rush Token its controller happens to own.
//!
//! THE GLOW (R662). The trap lights up on its controller's field while they have an empty, unlocked,
//! unreserved unit zone, R430's "if you have an empty unit zone" read by the same `open_zones` as
//! `match_` below: the half of the trigger the board decides. The play that sets it off is the other
//! half and has not happened yet, so the glow is the same on both faces.

use jackioh_engine::catalog::def_of;
use jackioh_engine::effects::{fill_board, forced_attacks, summon};
use jackioh_engine::prelude::*;
use jackioh_engine::zones::open_zones;

pub const ID: &str = "core-060";

/// §7's shared Rush Token, 3/3 with Rush.
const RUSH_TOKEN: &str = "core-t-rush";

/// §10.5 step 7's event: the play this trap answers, with the cost R56 reads. (TS
/// `Extract<GameEvent, { type: "cardResolved" }>`: here the fields of it this card reads after
/// `match_` has read the rest.)
#[derive(Clone, Debug)]
struct ResolvedPlay {
    instance_id: String,
    def_id: String,
    permanent: bool,
}

/// "When your opponent plays a (1) Cost or less card, if you have an empty unit zone" (radiant: any
/// card). Returns the play when the trap answers this event and `None` when it must stay armed.
/// (TS `match`, a Rust keyword: SURFACE §4.1's trailing underscore.)
fn match_(ctx: &EffectContext, event: &GameEvent, any_cost: bool) -> Option<ResolvedPlay> {
    let GameEvent::CardResolved {
        player,
        instance_id,
        def_id,
        permanent,
        cost_paid,
        ..
    } = event
    else {
        return None;
    };
    // "the opponent plays": the trap's own controller setting off their own trap is not the trigger.
    if *player == ctx.controller {
        return None;
    }
    // R56 and R70: the cost actually paid, so a cast (0) is always "1 or less".
    if !any_cost && *cost_paid > 1 {
        return None;
    }
    // R430: "if you have an empty unit zone" — an empty, unlocked, unreserved one (R64).
    if open_zones(&ctx.state, ctx.controller, Row::Units).is_empty() {
        return None;
    }
    Some(ResolvedPlay {
        instance_id: instance_id.clone(),
        def_id: def_id.clone(),
        permanent: *permanent,
    })
}

/// "summon 2 Rush Tokens" / "fill your board with Rush Tokens"; then "if it was a Unit, they attack
/// it". A played Spell, Field Spell, Trap or Field Trap leaves the tokens standing and attacks
/// nothing. The Unit test reads the def rather than the board, so a played unit that died during its
/// own resolution still counts as a Unit — but "it" is the played card's stay on the field (R174),
/// so once that has ended the tokens attack nothing: `permanent` says so, including when an earlier
/// trap answering the same play took the card off the field and Reborn brought a new body back
/// (`traps.rs` reads the flag again for each trap).
fn tokens_and_attack(ctx: &EffectContext, played: &ResolvedPlay, fill: bool) -> Vec<Effect> {
    let mut tokens: Vec<Effect> = if fill {
        vec![fill_board(json_as(json!({ "defId": RUSH_TOKEN })))]
    } else {
        vec![
            summon(json_as(json!({ "defId": RUSH_TOKEN }))),
            summon(json_as(json!({ "defId": RUSH_TOKEN }))),
        ]
    };

    if def_of(Some(&*ctx.state), &played.def_id).type_ != CardType::Unit || !played.permanent {
        return tokens;
    }
    tokens.push(forced_attacks(json_as(json!({
        "attackers": { "side": "self", "defId": RUSH_TOKEN, "summonedThisScript": true },
        "target": { "instanceId": played.instance_id },
    }))));
    tokens
}

/// One face's trigger. TS's `TrapTrigger` is `TriggerDef` plus the `when` predicate traps.ts reads,
/// and the whole condition lives in that predicate (R99, R61): `run` is reached only once the trap
/// really is firing, so the `None` branch below is narrowing and never a decision — a 2-cost play
/// has already been declined by `when` and left the trap armed and face-down.
fn honeypot(any_cost: bool, fill: bool) -> TriggerDef {
    TriggerDef::new("bear-honeypot", &[GameEventType::CardResolved], move |ctx, event| {
        match match_(ctx, event, any_cost) {
            None => vec![],
            Some(played) => tokens_and_attack(ctx, &played, fill),
        }
    })
    .with_when(move |ctx, event| match_(ctx, event, any_cost).is_some())
}

/// R662, R430: armed while its controller has an open unit zone for the tokens.
fn condition_met() -> ConditionHook {
    condition_hook(|ctx| {
        ctx.zone == ConditionZone::Field && !open_zones(ctx.state, ctx.controller, Row::Units).is_empty()
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            triggers: vec![honeypot(false, false)],
            condition_met: Some(condition_met()),
            ..Script::default()
        },
        radiant: Script {
            triggers: vec![honeypot(true, true)],
            condition_met: Some(condition_met()),
            ..Script::default()
        },
    }
}

// #60 Bear Honeypot (SPEC §8.3, BUILD M4-T4 row 60: "Fires after the opponent's ≤1-cost play
// resolves (R17, R56); a unit is attacked by each token in order until dead, one combat each (R53);
// radiant any card and fills the board"), and patch v0.2.0's condition (R430): "if you have an empty
// unit zone" — while its controller's unit row has no empty, unlocked, unreserved zone the trap does
// not fire and is not consumed.
//
// Every test here puts the trap face-down in p1's backrow and makes p2 the active player, which is
// the whole point of a Trap: it fires on the OPPONENT's turn and resolves to completion before their
// action continues (§10.3). Bear Honeypot opens no prompt, so the "a trap may prompt its own
// controller" path of §10.3 has nothing to exercise here.
//
// Rulings proved here: R17 (this trap fires after the played card has resolved, the moment #41
// Sheepish shares since R427), R56 (the threshold reads the cost actually paid), R70 (a cast pays 0,
// so it is "costing 1 or less"), R53 (forced attacks in full), R64 ("fill your board" is every empty
// unlocked unit zone, left to right), R11 (a dead unit token ceases to exist), R430 (no empty unit
// zone: the trap waits), §5.1 (a Trap is consumed when it fires).
//
// R662's yellow glow (`conditionMet`): on its controller's field while they have an open unit zone,
// both faces, checked against the opponent's play, at the end of this file.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// The trap, face-down in p1's backrow lane 3 — a Trap is only ever face-down until it fires (§5.1).
    fn armed(radiant: bool) -> Value {
        let mut card = json!({ "def": "core-060", "faceUp": false, "lane": 3 });
        if radiant {
            card["radiant"] = json!(true);
        }
        card
    }

    fn units_of(g: &Scenario, player: PlayerId) -> Vec<String> {
        (1..=5)
            .filter_map(|lane| g.unit(player, lane).map(|unit| unit.def_id))
            .collect()
    }

    /// How many events of this type the whole game has emitted (`"trapFired" | "attackDeclared" |
    /// "summoned"`).
    fn count_of(g: &Scenario, kind: &str) -> usize {
        g.events()
            .iter()
            .filter(|event| event.event_type().as_str() == kind)
            .count()
    }

    /// p1 needs no hand: it is not their turn. p2 keeps a spare card so R82 never auto-ends the turn.
    fn spare() -> Value {
        json!({ "hand": ["core-005"], "library": ["core-011", "core-016"] })
    }

    /// TS `{ ...base, ...extra }` over two JSON objects: the later keys win.
    fn spread(base: Value, extra: Value) -> Value {
        let mut all = base.as_object().cloned().unwrap_or_default();
        for (key, value) in extra.as_object().cloned().unwrap_or_default() {
            all.insert(key, value);
        }
        Value::Object(all)
    }

    /// `s.backrow(p, lane)?.faceUp`.
    fn face_up_at(g: &Scenario, player: PlayerId, lane: i32) -> Option<bool> {
        g.backrow(player, lane).and_then(|card| card.face_up)
    }

    fn def_at(g: &Scenario, player: PlayerId, lane: i32) -> Option<String> {
        g.unit(player, lane).map(|unit| unit.def_id)
    }

    mod base_when_it_fires {
        use super::*;

        #[test]
        fn r56_fires_on_the_opponent_s_1_cost_play_and_summons_2_rush_tokens() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)] },
                "p2": spread(spare(), json!({ "hand": ["core-015", "core-005"] })),
            }));

            g.play("core-015", json!({ "zone": 1 }));

            g.expect_events(json!(["cardPlayed", "trapFired", "summoned"]));
            assert_eq!(units_of(&g, P1), ["core-t-rush", "core-t-rush"]);
        }

        #[test]
        fn r56_a_2_cost_play_does_not_fire_it_the_trap_stays_armed_and_face_down() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)] },
                "p2": spread(spare(), json!({ "hand": ["core-020", "core-005"] })),
            }));

            g.play("core-020", json!({ "zone": 1 }));

            assert_eq!(count_of(&g, "trapFired"), 0);
            assert!(units_of(&g, P1).is_empty());
            g.expect_in_zone("core-060", "field");
            assert_eq!(face_up_at(&g, P1, 3), Some(false));
        }

        #[test]
        fn r70_a_cast_pays_0_so_a_cast_card_is_costing_1_or_less() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)] },
                // #21 Hinder is cost 0 and cast on draw: drawing it casts it (§2.4, R58). Its Radiant face
                // discards nothing (R431), so the cast asks no question.
                "p2": { "hand": ["core-005"], "library": [{ "def": "core-021", "radiant": true }, "core-011"] },
            }));

            g.start_turn();

            g.expect_events(json!(["drawn", "cardPlayed", "trapFired", "summoned"]));
            assert_eq!(units_of(&g, P1), ["core-t-rush", "core-t-rush"]);
        }

        #[test]
        fn the_opponent_plays_the_trap_s_own_controller_never_sets_it_off() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": spread(spare(), json!({ "backrow": [armed(false)], "hand": ["core-011", "core-005"] })),
                "p2": spare(),
            }));

            g.play("core-011", json!({ "zone": 1 }));

            assert_eq!(count_of(&g, "trapFired"), 0);
            g.expect_in_zone("core-060", "field");
        }

        #[test]
        fn s5_1_a_trap_is_consumed_when_it_fires_and_goes_to_its_owner_s_graveyard() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)] },
                "p2": spread(spare(), json!({ "hand": ["core-005", "core-011"] })),
            }));

            g.play("core-005", json!({}));

            g.expect_events(json!(["trapFired", "enteredGraveyard"]))
                .expect_in_zone("core-060", "graveyard");
            assert_eq!(count_of(&g, "trapFired"), 1);
        }
    }

    mod r430_only_with_an_empty_unit_zone {
        use super::*;

        /// p1's unit row, lanes 1–5, filled with #19s except the lanes given.
        fn row(except: &[i32]) -> Value {
            Value::Array(
                (1..=5)
                    .filter(|lane| !except.contains(lane))
                    .map(|lane| json!({ "def": "core-019", "lane": lane }))
                    .collect(),
            )
        }

        #[test]
        fn r430_a_full_unit_row_the_1_cost_play_resolves_and_the_trap_neither_fires_nor_is_consumed() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)], "field": row(&[]) },
                "p2": spread(spare(), json!({ "hand": ["core-015", "core-005"] })),
            }));

            g.play("core-015", json!({ "zone": 1 }));

            assert_eq!(count_of(&g, "trapFired"), 0);
            assert_eq!(count_of(&g, "attackDeclared"), 0);
            g.expect_in_zone("core-060", "field");
            assert_eq!(face_up_at(&g, P1, 3), Some(false));
            assert_eq!(units_of(&g, P2), ["core-015", "core-t-rush"]);
        }

        #[test]
        fn r430_a_locked_zone_is_not_an_empty_unit_zone_the_trap_waits() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)], "field": row(&[5]) },
                "p2": spread(spare(), json!({ "hand": ["core-015", "core-005"] })),
            }));
            g.state_mut().players.p1.locks.units[4] = true;

            g.play("core-015", json!({ "zone": 1 }));

            assert_eq!(count_of(&g, "trapFired"), 0);
            g.expect_in_zone("core-060", "field");
        }

        #[test]
        fn r430_r64_a_zone_a_dying_reborn_unit_holds_is_not_an_empty_unit_zone_either() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)], "field": row(&[5]) },
                "p2": spread(spare(), json!({ "hand": ["core-015", "core-005"] })),
            }));
            g.state_mut().reserved.push(ZoneRef {
                player: P1,
                row: Row::Units,
                lane: 5,
            });

            g.play("core-015", json!({ "zone": 1 }));

            assert_eq!(count_of(&g, "trapFired"), 0);
            g.expect_in_zone("core-060", "field");
        }

        #[test]
        fn r430_r64_one_empty_zone_is_enough_it_fires_the_one_token_that_fits_arrives_and_attacks_the_other_fizzles() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)], "field": row(&[4]) },
                "p2": spread(spare(), json!({ "hand": ["core-015", "core-005"] })),
            }));

            g.play("core-015", json!({ "zone": 1 }));

            assert_eq!(count_of(&g, "trapFired"), 1);
            assert_eq!(def_at(&g, P1, 4).as_deref(), Some("core-t-rush"));
            assert_eq!(units_of(&g, P1).iter().filter(|def_id| *def_id == "core-t-rush").count(), 1);
            g.expect_in_zone("core-060", "graveyard");
            // The played 1/1 met the one token and died to it.
            assert_ne!(def_at(&g, P2, 1).as_deref(), Some("core-015"));
        }

        #[test]
        fn r430_a_trap_that_waited_fires_on_a_later_play_once_a_zone_is_free() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)], "field": row(&[]) },
                "p2": spread(spare(), json!({ "hand": ["core-015", "core-016", "core-005", "core-011"], "mana": 10 })),
            }));
            g.play("core-015", json!({ "zone": 1 }));
            assert_eq!(count_of(&g, "trapFired"), 0);

            // #16 Hit Job (3) opens p1's lane 2; it costs too much to set the trap off itself.
            let menace = g.unit(P1, 2);
            assert!(menace.is_some());
            let menace_id = menace.map(|unit| unit.id).unwrap_or_default();
            g.play("core-016", json!({ "targets": [{ "pick": "instance", "instanceId": menace_id }] }));
            assert_eq!(count_of(&g, "trapFired"), 0);

            // #5 Stockpile (1) now finds the room: the trap fires, one token fits, and a Spell is attacked by nothing.
            g.play("core-005", json!({}));

            assert_eq!(count_of(&g, "trapFired"), 1);
            assert_eq!(def_at(&g, P1, 2).as_deref(), Some("core-t-rush"));
            assert_eq!(count_of(&g, "attackDeclared"), 0);
            g.expect_in_zone("core-060", "graveyard");
        }

        #[test]
        fn r430_the_radiant_face_waits_on_a_full_row_too() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(true)], "field": row(&[]) },
                "p2": spread(spare(), json!({ "hand": ["core-020", "core-005"] })),
            }));

            g.play("core-020", json!({ "zone": 1 }));

            assert_eq!(count_of(&g, "trapFired"), 0);
            g.expect_in_zone("core-060", "field");
            assert_eq!(face_up_at(&g, P1, 3), Some(false));
        }
    }

    mod base_r17_s_timing {
        use super::*;

        #[test]
        fn r17_fires_after_the_played_card_has_resolved_so_its_cry_has_already_happened() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)] },
                // #15 Me and Mr Token is a 1-cost Unit whose Cry summons a Rush Token for ITS controller.
                "p2": spread(spare(), json!({ "hand": ["core-015", "core-005"] })),
            }));

            g.play("core-015", json!({ "zone": 1 }));

            // cardPlayed → Mr Token summoned → its Cry's token summoned → only then trapFired.
            g.expect_events(json!(["cardPlayed", "summoned", "summoned", "trapFired", "summoned"]));
            // The Cry's token is p2's and was never the forced-attack target.
            assert!(units_of(&g, P2).iter().any(|def_id| def_id == "core-t-rush"));
        }

        #[test]
        fn the_engine_cell_the_played_unit_is_on_the_field_when_the_tokens_arrive() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)] },
                "p2": spread(spare(), json!({ "hand": ["core-015", "core-005"] })),
            }));

            g.play("core-015", json!({ "zone": 1 }));

            // It was attacked, which is only possible because it was on the field (§4.2 step 2).
            g.expect_events(json!(["trapFired", "attackDeclared"]));
        }
    }

    mod base_the_forced_attacks_r53 {
        use super::*;

        #[test]
        fn r53_each_forced_attack_is_its_own_combat_and_the_second_token_does_not_attack_a_dead_target() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)] },
                // Me and Mr Token is 1/1: one 3-damage hit kills it, and it strikes back for 1.
                "p2": spread(spare(), json!({ "hand": ["core-015", "core-005"] })),
            }));

            g.play("core-015", json!({ "zone": 1 }));

            assert_eq!(count_of(&g, "attackDeclared"), 1);
            g.expect_in_zone("core-015", "graveyard");
            assert_eq!(units_of(&g, P1), ["core-t-rush", "core-t-rush"]);
        }

        #[test]
        fn r53_the_target_still_strikes_back() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)] },
                "p2": spread(spare(), json!({ "hand": ["core-015", "core-005"] })),
            }));

            g.play("core-015", json!({ "zone": 1 }));

            let first = g.unit(P1, 1);
            let second = g.unit(P1, 2);
            assert!(first.is_some());
            assert!(second.is_some());
            let (Some(first), Some(second)) = (first, second) else {
                return;
            };

            // 1 damage came back onto the attacker; the token that never attacked is untouched.
            g.expect_stats(&first, json!({ "health": 2, "maxHealth": 3 }));
            g.expect_stats(&second, json!({ "health": 3, "maxHealth": 3 }));
        }

        #[test]
        fn r53_a_forced_attack_spends_no_exertion_and_ignores_summoning_sickness() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)] },
                "p2": spread(spare(), json!({ "hand": ["core-015", "core-005"] })),
            }));

            g.play("core-015", json!({ "zone": 1 }));

            // The tokens were summoned this very turn and attacked anyway (sickness skipped), and the
            // attack cost them nothing, so their own turn is still ahead of them.
            let first = g.unit(P1, 1);
            assert_eq!(
                first.map(|unit| serde_json::to_value(unit.exertion).expect("exertion serialises")),
                Some(json!({ "attacked": false, "switched": false }))
            );
        }

        #[test]
        fn r53_a_token_that_dies_to_the_strike_back_ceases_to_exist_r11_and_the_run_stops() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)] },
                // Pointmaster is 7/2 with First Strike, so it kills the first token outright — but it is a
                // 2-cost card, so only the radiant face reaches it. Tempo Timmy is the 1-cost First Strike
                // unit: 3/3, it strikes first for 3 and the token deals nothing back (§4.3).
                "p2": spread(spare(), json!({ "hand": ["core-011", "core-005"] })),
            }));

            g.play("core-011", json!({ "zone": 1 }));

            let survivors = units_of(&g, P1);
            // Both tokens attack: each dies to First Strike without hurting Timmy, so Timmy stands.
            assert_eq!(count_of(&g, "attackDeclared"), 2);
            assert!(survivors.is_empty());
            g.expect_in_zone("core-011", "field")
                .expect_stats("core-011", json!({ "health": 3, "maxHealth": 3 }));
        }

        #[test]
        fn a_played_spell_summons_the_tokens_and_nothing_attacks() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(false)] },
                "p2": spread(spare(), json!({ "hand": ["core-005", "core-011"] })),
            }));

            g.play("core-005", json!({}));

            assert_eq!(units_of(&g, P1), ["core-t-rush", "core-t-rush"]);
            assert_eq!(count_of(&g, "attackDeclared"), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn any_card_a_3_cost_play_fires_it() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(true)] },
                // Mana Well is a 3-cost Field Spell: no cost threshold left, and not a Unit, so no attacks.
                "p2": spread(spare(), json!({ "hand": ["core-006", "core-005"] })),
            }));

            g.play("core-006", json!({ "zone": 1 }));

            g.expect_events(json!(["cardPlayed", "trapFired", "summoned"]));
            assert_eq!(count_of(&g, "attackDeclared"), 0);
        }

        #[test]
        fn r64_fills_your_board_every_empty_unlocked_unit_zone_left_to_right() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(true)] },
                "p2": spread(spare(), json!({ "hand": ["core-006", "core-005"] })),
            }));

            g.play("core-006", json!({ "zone": 1 }));

            assert_eq!(units_of(&g, P1), vec!["core-t-rush"; 5]);
            for lane in [1, 2, 3, 4, 5] {
                assert_eq!(def_at(&g, P1, lane).as_deref(), Some("core-t-rush"));
            }
        }

        #[test]
        fn r64_fills_only_the_empty_zones_leaving_what_is_already_there() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(true)], "field": [{ "def": "core-008", "lane": 1 }] },
                "p2": spread(spare(), json!({ "hand": ["core-006", "core-005"] })),
            }));

            g.play("core-006", json!({ "zone": 1 }));

            assert_eq!(
                units_of(&g, P1),
                ["core-008", "core-t-rush", "core-t-rush", "core-t-rush", "core-t-rush"]
            );
        }

        #[test]
        fn s8_conventions_same_a_played_unit_is_still_attacked_one_combat_at_a_time_r53() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(true)] },
                // Big D-fender is a 2-cost 0/8: three 3-damage combats kill it, and R63 makes its 0-attack
                // strike-back no damage instance at all, so every token survives to take its turn in the run.
                "p2": spread(spare(), json!({ "hand": ["core-001", "core-005"] })),
            }));

            g.play("core-001", json!({ "zone": 1 }));

            // Combats 1 and 2 leave it at 2 health, combat 3 kills it, and tokens 4 and 5 never attack
            // because the target has left the field (R53's last clause).
            assert_eq!(count_of(&g, "attackDeclared"), 3);
            g.expect_in_zone("core-001", "graveyard");
            assert_eq!(units_of(&g, P1), vec!["core-t-rush"; 5]);
        }

        #[test]
        fn s4_3_first_strike_on_the_target_kills_each_forced_attacker_before_it_lands_its_hit() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(true)] },
                // Pointmaster is a 2-cost 7/1 with First Strike: 7 into a 3-health token, and "if D is
                // destroyed here it deals nothing", so all five tokens die and Pointmaster stands at 1.
                "p2": spread(spare(), json!({ "hand": ["core-020", "core-005"] })),
            }));

            g.play("core-020", json!({ "zone": 1 }));

            assert_eq!(count_of(&g, "attackDeclared"), 5);
            assert!(units_of(&g, P1).is_empty());
            g.expect_in_zone("core-020", "field")
                .expect_stats("core-020", json!({ "health": 1, "maxHealth": 1 }));
        }

        #[test]
        fn r56_the_radiant_face_keeps_no_threshold_but_still_only_answers_the_opponent() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": spread(spare(), json!({ "backrow": [armed(true)], "hand": ["core-006", "core-005"] })),
                "p2": spare(),
            }));

            g.play("core-006", json!({ "zone": 1 }));

            assert_eq!(count_of(&g, "trapFired"), 0);
            g.expect_in_zone("core-060", "field");
        }

        #[test]
        fn s5_1_the_radiant_face_is_still_a_trap_not_a_field_trap_it_is_consumed() {
            crate::register_all();
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [armed(true)] },
                "p2": spread(spare(), json!({ "hand": ["core-006", "core-005"] })),
            }));

            g.play("core-006", json!({ "zone": 1 }));

            g.expect_in_zone("core-060", "graveyard");
        }
    }

    // TS: one `it` per face in a `for (const radiant of [false, true])` loop, titled with the face.
    mod glows_while_its_controller_has_an_open_unit_zone_r662_r430 {
        use super::*;

        const VANILLA: &str = "core-008"; // Mr. Vanilla, a 1-cost Unit
        const FULL: [&str; 5] = [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA];

        fn face(radiant: bool) -> &'static str {
            if radiant { "radiant" } else { "base" }
        }

        /// R662 `${face}`: with an open zone it glows for its controller only, and the opponent's cheap
        /// play sets it off.
        fn glows_with_an_open_zone(radiant: bool) {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": format!("r662-060-{}-on", face(radiant)),
                "active": "p2",
                "p1": { "backrow": [{ "def": "core-060", "radiant": radiant }] },
                "p2": { "hand": [VANILLA, "core-010"] },
            }));
            assert!(backrow_glows(&s, 1, P1));
            assert!(!opponent_sees_glow(&s, 1, P1));

            s.play(VANILLA, json!({}));
            assert!(s.events().iter().any(|event| event.event_type() == GameEventType::TrapFired));
        }

        /// R662 `${face}`: with every unit zone taken it does not glow, and the play leaves it armed (R430).
        fn dark_with_every_zone_taken(radiant: bool) {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": format!("r662-060-{}-off", face(radiant)),
                "active": "p2",
                "p1": { "backrow": [{ "def": "core-060", "radiant": radiant }], "field": FULL },
                "p2": { "hand": [VANILLA, "core-010"] },
            }));
            assert!(!backrow_glows(&s, 1, P1));

            s.play(VANILLA, json!({}));
            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::TrapFired));
            assert_eq!(s.backrow(P1, 1).map(|card| card.def_id).as_deref(), Some("core-060"));
        }

        #[test]
        fn r662_base_with_an_open_zone_it_glows_for_its_controller_only_and_the_opponent_s_cheap_play_sets_it_off() {
            glows_with_an_open_zone(false);
        }

        #[test]
        fn r662_radiant_with_an_open_zone_it_glows_for_its_controller_only_and_the_opponent_s_cheap_play_sets_it_off() {
            glows_with_an_open_zone(true);
        }

        #[test]
        fn r662_base_with_every_unit_zone_taken_it_does_not_glow_and_the_play_leaves_it_armed_r430() {
            dark_with_every_zone_taken(false);
        }

        #[test]
        fn r662_radiant_with_every_unit_zone_taken_it_does_not_glow_and_the_play_leaves_it_armed_r430() {
            dark_with_every_zone_taken(true);
        }
    }
}
