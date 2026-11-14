//! S #23 Warp Disruptor (SPEC §8.10 row 23; R1849006, R1849007, R1875003): (1) Field Spell, Rare, of
//! the Stinkyscape set.
//!   Base:    "Stack, Brittle 3 / Aura: Units can't be in Defense Position and have +{attack} Attack. /
//!            After your opponent summons a Unit, a random Unit of yours attacks it."
//!   Radiant: the same, and "Your Units have First Strike."
//! Stack, Brittle 3 (§6.1, R638). unitsNeverDefense keeps units out of Defense (R1849006); +attack aura (§10.4, R1875003).
//! Forced attack on opponent summons (C #5, R1875003, R1849007, R53). Radiant grants First Strike.
//! The end (REVIEW A3.6, R3800004): it answers each card once a turn, remembering whom under the turn's
//! number, so a body that resummons itself as it dies (S #112 Job Application's Death, fused) and dies to
//! the attack it answered returns once and stays.

use jackioh_engine::effects::{forced_attacks, remember};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-023";

/// R1875003: the opponent's Unit this event brings onto the field, or None: a summon of a Unit onto
/// their side (Recruit, a token, a copy, Reborn), or a played one once it resolves — its play's own
/// placement (§10.5 step 4, which carries `exitsFrom`) waits for that, as C #5 Tesla's does.
fn arrival(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<String> {
    let state: &GameState = &*ctx.state;
    match event {
        GameEvent::Summoned {
            player,
            row,
            exits_from,
            instance_id,
            ..
        } => {
            if *player == ctx.controller || exits_from.is_some() {
                return None;
            }
            // R1081000: a Unit summoned standing in a backrow zone is a summoned Unit too.
            let unit = find_instance(state, instance_id)?;
            (*row == Row::Units || stands_in_backrow(state, unit)).then(|| instance_id.clone())
        }
        GameEvent::CardResolved {
            permanent, instance_id, ..
        } => {
            // R783005: the play as it counts, so a Unit set by its Cloaked is the Trap it was set as.
            if !*permanent || played_type_of(state, event) != Some(CardType::Unit) {
                return None;
            }
            let unit = find_instance(state, instance_id)?;
            (unit.zone.z() == ZoneName::Field && unit.controller != ctx.controller).then(|| instance_id.clone())
        }
        _ => None,
    }
}

/// "Aura: Units … have +{attack} Attack", every Unit of both sides; on the Radiant face also "Your
/// Units have First Strike".
fn disruptor_aura(radiant: bool) -> AuraHook {
    aura_hook(move |a| {
        let amount = param(&a, "attack");
        let controller = a.self_.controller;
        let mut entries = vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| in_units_row_or_standing(a.state, unit)),
            mod_: StatMod {
                attack: Some(amount),
                ..StatMod::default()
            },
        }];
        if radiant {
            entries.push(AuraEntry {
                applies: Box::new(move |unit: &CardInstance| {
                    unit.controller == controller && in_units_row_or_standing(a.state, unit)
                }),
                mod_: StatMod {
                    keywords: Some(vec![Keyword::FirstStrike]),
                    ..StatMod::default()
                },
            });
        }
        entries
    })
}

/// R3800004: what a Warp Disruptor keeps of the cards it has answered: `{ "turn", "ids" }`, this turn's
/// alone counting, so a memory from another turn or another stay (R78) holds no one.
const ANSWERED: &str = "answered";

/// R3800004: the cards this Warp Disruptor has answered this turn (S #24's too).
pub fn answered_this_turn(ctx: &EffectContext<'_>) -> Vec<String> {
    let Some(memory) = recalled(ctx, ANSWERED) else {
        return Vec::new();
    };
    if memory.get("turn").and_then(Value::as_i64) != Some(i64::from(ctx.state.turn)) {
        return Vec::new();
    }
    memory
        .get("ids")
        .and_then(Value::as_array)
        .map(|ids| ids.iter().filter_map(Value::as_str).map(String::from).collect())
        .unwrap_or_default()
}

/// R3800004: the arrival a Warp Disruptor answers, `arrival`'s unless it has answered that card this turn.
pub fn unanswered(ctx: &EffectContext<'_>, arrival: Option<String>) -> Option<String> {
    arrival.filter(|id| !answered_this_turn(ctx).contains(id))
}

/// R3800004: the memory that marks `target` answered this turn, before the forced attack it answers with.
pub fn remember_answered(ctx: &EffectContext<'_>, target: &str) -> Effect {
    let mut ids = answered_this_turn(ctx);
    ids.push(target.to_string());
    remember(json_as(json!({
        "key": ANSWERED,
        "value": { "turn": ctx.state.turn, "ids": ids },
    })))
}

/// The answer: `once` (R3800004) lets it answer each card once a turn; without it, as through v0.6.1b.
fn disruptor_with(radiant: bool, once: bool) -> Script {
    let answer = TriggerDef::new(
        "warp-disruptor",
        &[GameEventType::Summoned, GameEventType::CardResolved],
        move |ctx, event| {
            let found = arrival(ctx, event);
            let Some(target) = (if once { unanswered(ctx, found) } else { found }) else {
                return Vec::new();
            };
            let attack = forced_attacks(json_as(json!({
                "attackers": { "side": "self", "random": true },
                "target": { "instanceId": target },
            })));
            if once {
                vec![remember_answered(ctx, &target), attack]
            } else {
                vec![attack]
            }
        },
    )
    .with_when(move |ctx, event| {
        let found = arrival(ctx, event);
        if once { unanswered(ctx, found) } else { found }.is_some()
    });
    Script {
        static_flags: Some(StaticFlags {
            units_never_defense: Some(true),
            ..StaticFlags::default()
        }),
        aura: Some(disruptor_aura(radiant)),
        triggers: vec![answer],
        ..Script::default()
    }
}

fn disruptor(radiant: bool) -> Script {
    disruptor_with(radiant, true)
}

pub fn script() -> CardScripts {
    CardScripts {
        base: disruptor(false),
        radiant: disruptor(true),
    }
}

/// R767000, R3800004: through v0.6.1b it answered every arrival, a card it had answered this turn
/// included; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_1b() -> CardScripts {
    CardScripts {
        base: disruptor_with(false, false),
        radiant: disruptor_with(true, false),
    }
}

// S #23 Warp Disruptor — SPEC §8.10 row 23, BUILD M12 row S 23 (R638).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;

    const FILLER: &str = "core-005"; // Stockpile: a Spell, a card in hand, so no turn ends by itself
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const MOTHS: &str = "core-009"; // Moths to the Flame
    const STRATOS: &str = "stinkyscape-068"; // Stratos: Flying, Ranged
    const GREASER: &str = "stand-in (2) 7/7 R21/21";
    const ATTACK: i32 = 2;
    const NO_DEFENSE: &str = "cannot be in Defense Position";

    fn played(radiant: bool, mine: Value, theirs: Value, hand: Value) -> Scenario {
        let mut s = crate::scenario(json!({
            "seed": "warp-disruptor",
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": mine,
                "library": crate::filler(6),
                "mana": 10,
            },
            "p2": { "hand": hand, "field": theirs, "library": crate::filler(6), "mana": 10 },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    fn forced(s: &Scenario) -> Vec<(String, String)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AttackDeclared {
                    attacker_id,
                    target_id,
                    forced: true,
                    ..
                } => Some((attacker_id.clone(), target_id.clone())),
                _ => None,
            })
            .collect()
    }

    fn their_turn(s: &mut Scenario) {
        s.end_turn();
        s.state_mut().players[P2].mana.current = 10;
    }

    mod base {
        use super::*;

        #[test]
        fn r1849006_no_unit_of_either_side_switches_to_defense() {
            let mut s = played(false, json!([VANILLA]), json!([VANILLA]), json!([FILLER]));
            let mine = s.unit_at(P1, 1);
            s.expect_refused_with(|s| s.switch_position(&mine), NO_DEFENSE);
            their_turn(&mut s);
            let theirs = s.unit_at(P2, 1);
            s.expect_refused_with(|s| s.switch_position(&theirs), NO_DEFENSE);
        }

        #[test]
        fn r1849006_a_unit_in_defense_switches_to_attack_as_it_arrives() {
            let s = played(
                false,
                json!([{ "def": MOTHS, "position": "DEF" }]),
                json!([{ "def": MOTHS, "position": "DEF" }]),
                json!([FILLER]),
            );
            assert_eq!(s.unit_at(P1, 1).position, Some(Position::Atk));
            assert_eq!(s.unit_at(P2, 1).position, Some(Position::Atk));
        }

        #[test]
        fn r1875003_every_unit_of_both_sides_has_2_more_attack() {
            let s = played(false, json!([VANILLA]), json!([VANILLA]), json!([FILLER]));
            assert_eq!(s.stats(s.unit_at(P1, 1).id.as_str()).attack, 4 + ATTACK);
            assert_eq!(s.stats(s.unit_at(P2, 1).id.as_str()).attack, 4 + ATTACK);
        }

        /// R783005: a Unit set face-down by its Cloaked is a Trap play, no Unit played.
        #[test]
        fn r783005_a_unit_set_face_down_by_its_cloaked_is_not_attacked() {
            let mut s = played(
                false,
                json!([{ "def": MOTHS, "lane": 1 }, { "def": MOTHS, "lane": 2 }]),
                json!([]),
                json!([crate::CLOAKED_UNIT, FILLER]),
            );
            their_turn(&mut s);
            crate::set_cloaked(&mut s, P2, crate::CLOAKED_UNIT, 3);
            assert!(forced(&s).is_empty(), "{:?}", forced(&s));
        }

        #[test]
        fn r1875003_a_played_enemy_unit_is_attacked_once_it_resolves_by_one_random_unit() {
            let mut s = played(
                false,
                json!([{ "def": MOTHS, "lane": 1 }, { "def": MOTHS, "lane": 2 }]),
                json!([]),
                json!([VANILLA, FILLER]),
            );
            let mine = [s.unit_at(P1, 1).id, s.unit_at(P1, 2).id];
            their_turn(&mut s);
            let cursor = s.state().rng_cursor;
            s.play(VANILLA, json!({ "zone": 3 }));
            let attacks = forced(&s);
            assert_eq!(attacks.len(), 1, "one of them attacks");
            assert!(mine.contains(&attacks[0].0));
            assert_eq!(attacks[0].1, s.card(VANILLA).id);
            let types: Vec<GameEventType> = s.last_events().iter().map(GameEvent::event_type).collect();
            let resolved = types.iter().position(|t| *t == GameEventType::CardResolved);
            let declared = types.iter().position(|t| *t == GameEventType::AttackDeclared);
            assert!(resolved < declared, "{types:?}");
            assert_eq!(s.state().rng_cursor, cursor + 1);
        }

        #[test]
        fn r1849007_a_flying_arrival_draws_only_a_flying_attacker() {
            let mut s = played(
                false,
                json!([{ "def": VANILLA, "lane": 1 }, { "def": STRATOS, "lane": 2 }, { "def": VANILLA, "lane": 3 }]),
                json!([]),
                json!([STRATOS, FILLER]),
            );
            let flyer = s.unit_at(P1, 2).id;
            their_turn(&mut s);
            s.play(STRATOS, json!({ "zone": 1 }));
            assert_eq!(forced(&s).into_iter().map(|(attacker, _)| attacker).collect::<Vec<_>>(), vec![flyer]);
        }

        #[test]
        fn r1849007_with_none_able_nothing_happens_and_the_rng_does_not_move() {
            let mut s = played(false, json!([VANILLA]), json!([]), json!([STRATOS, FILLER]));
            their_turn(&mut s);
            let cursor = s.state().rng_cursor;
            s.play(STRATOS, json!({ "zone": 1 }));
            assert!(forced(&s).is_empty());
            assert_eq!(s.state().rng_cursor, cursor);
        }

        #[test]
        fn its_own_sides_play_is_not_answered() {
            let mut s = crate::scenario(json!({
                "p1": {
                    "hand": [ID, VANILLA, FILLER],
                    "field": [{ "def": VANILLA, "lane": 1 }],
                    "library": crate::filler(4),
                    "mana": 10,
                },
                "p2": { "field": [{ "def": VANILLA, "lane": 1 }], "library": crate::filler(4) },
            }));
            s.play(ID, json!({ "zone": 1 }));
            s.play(VANILLA, json!({ "zone": 2 }));
            assert!(forced(&s).is_empty());
        }

        #[test]
        fn r638_brittle_3_destroys_it_in_time() {
            let mut s = played(false, json!([]), json!([]), json!([FILLER]));
            for _ in 0..3 {
                crate::next_turn_of(&mut s, P1);
            }
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn attack_reads_through_param() {
            let mut s = played(false, json!([VANILLA]), json!([]), json!([FILLER]));
            let disruptor = s.backrow(P1, 1).expect("played").id;
            assert_eq!(crate::upgrade_number(&mut s, &disruptor, "attack"), ATTACK + 1);
            assert_eq!(s.stats(s.unit_at(P1, 1).id.as_str()).attack, 4 + ATTACK + 1);
        }
    }

    mod radiant {
        use super::*;

        fn first_strike(s: &Scenario, id: &str) -> bool {
            unit_has(s.state(), s.card(id), KeywordKind::FirstStrike)
        }

        #[test]
        fn r1875003_its_controllers_units_have_first_strike_and_the_opponents_do_not() {
            let s = played(true, json!([VANILLA]), json!([VANILLA]), json!([FILLER]));
            assert!(first_strike(&s, &s.unit_at(P1, 1).id));
            assert!(!first_strike(&s, &s.unit_at(P2, 1).id));
            assert_eq!(s.stats(s.unit_at(P2, 1).id.as_str()).attack, 4 + ATTACK);
        }

        #[test]
        fn r1875003_the_forced_attack_strikes_first() {
            let mut s = played(true, json!([GREASER]), json!([]), json!([VANILLA, FILLER]));
            let mine = s.unit_at(P1, 1).id;
            their_turn(&mut s);
            s.play(VANILLA, json!({ "zone": 1 }));
            s.expect_in_zone(VANILLA, "graveyard");
            s.expect_stats(mine.as_str(), json!({ "health": 7 }));
        }
    }

    /// R3800004: the end of the loop daily fuzz seed 9999 found: a Unit of the opponent's with S #112 Job
    /// Application's "Death: Resummon this at Stage 1" fused in died to a Radiant Warp Disruptor's forced
    /// attack, which its First Strike made safe, and every return was answered again.
    mod r3800004_it_answers_each_card_once_a_turn {
        use super::*;

        const JOB_APPLICATION: &str = "stinkyscape-112";

        /// P2's Unit with Job Application fused in, as `fuseCards` makes it.
        fn self_resummoning(s: &mut Scenario) -> String {
            let id = s.unit_at(P2, 1).id;
            crate::run_as(
                s,
                jackioh_engine::effects::fuse_cards(json_as(json!({
                    "instanceIds": [id],
                    "defIds": [JOB_APPLICATION],
                    "targetInstanceId": id,
                }))),
                P2,
            );
            assert!(s.card(id.as_str()).def_id.starts_with("t-"), "fused");
            id
        }

        fn deaths_of(s: &Scenario, id: &str) -> usize {
            s.last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if instance_id == id))
                .count()
        }

        /// Kills the body with an attack of P1's: it returns, is answered once, dies again and returns.
        fn kill(s: &mut Scenario, mine: &str, body: &str) {
            s.attack(mine, body);
            assert_eq!(deaths_of(s, body), 2, "it died to the attack and to the one answer");
            assert_eq!(forced(s), vec![(mine.to_string(), body.to_string())]);
            s.expect_in_zone(body, "field");
            assert!(s.state().result.is_none());
        }

        #[test]
        fn r3800004_a_self_resummoning_unit_it_kills_is_answered_once_and_stays() {
            let mut s = played(true, json!([GREASER]), json!([VANILLA]), json!([FILLER]));
            let mine = s.unit_at(P1, 1).id;
            let body = self_resummoning(&mut s);
            kill(&mut s, &mine, &body);
            s.expect_stats(mine.as_str(), json!({ "health": 7 }));
        }

        #[test]
        fn r3800004_a_card_answered_on_an_earlier_turn_is_answered_again() {
            let mut s = played(true, json!([GREASER]), json!([VANILLA]), json!([FILLER]));
            let mine = s.unit_at(P1, 1).id;
            let body = self_resummoning(&mut s);
            kill(&mut s, &mine, &body);
            s.end_turn();
            s.end_turn();
            kill(&mut s, &mine, &body);
        }
    }
}
