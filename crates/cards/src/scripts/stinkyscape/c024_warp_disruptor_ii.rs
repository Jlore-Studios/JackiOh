//! S #24 Warp Disruptor II (SPEC §8.10 row 24; R1849006, R1849007, R1875003): (2) Field Spell, Rare, of
//! the Stinkyscape set.
//!   Base:    "Stack, Brittle 5 / Aura: Units can't be in Defense Position and have +{attack} Attack. /
//!            After your opponent summons a Unit, each of your Units attacks it."
//!   Radiant: the same, and "Your Units have First Strike."
//! Engine: S #23 Warp Disruptor's, with +{attack} Attack, Brittle 5 (R638) and, after a Unit arrives on
//! the opponent's side (however summoned; a played one once it resolves, R1875003), a forced attack on
//! it by every one of its controller's Units that may attack it, in lane order, each its own combat and
//! state check, the run stopping once the arrival has left the field (`forced_attacks` without
//! `random`, R1849007, R53). Radiant: its controller's Units have First Strike too. Tunes: attack 3 ↑.
//! The end (REVIEW A3.6, R3800004): as S #23's, it answers each card once a turn.

use jackioh_engine::effects::forced_attacks;
use jackioh_engine::prelude::*;

use crate::scripts::stinkyscape::c023_warp_disruptor::{remember_answered, unanswered};

pub const ID: &str = "stinkyscape-024";

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

/// The answer: `once` (R3800004) lets it answer each card once a turn; without it, as through v0.6.1b.
fn disruptor_with(radiant: bool, once: bool) -> Script {
    let answer = TriggerDef::new(
        "warp-disruptor-ii",
        &[GameEventType::Summoned, GameEventType::CardResolved],
        move |ctx, event| {
            let found = arrival(ctx, event);
            let Some(target) = (if once { unanswered(ctx, found) } else { found }) else {
                return Vec::new();
            };
            let attack = forced_attacks(json_as(json!({
                "attackers": { "side": "self" },
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

// S #24 Warp Disruptor II — SPEC §8.10 row 24, BUILD M12 row S 24.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;

    const FILLER: &str = "core-005"; // Stockpile: a Spell, a card in hand, so no turn ends by itself
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const MOTHS: &str = "core-009"; // Moths to the Flame
    const GENN: &str = "stand-in (4) 14/14 R42/42";
    const GREASER: &str = "stand-in (2) 7/7 R21/21";
    const ATTACK: i32 = 3;
    const NO_DEFENSE: &str = "cannot be in Defense Position";

    fn played(radiant: bool, mine: Value, hand: Value) -> Scenario {
        let mut s = crate::scenario(json!({
            "seed": "warp-disruptor-ii",
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": mine,
                "library": crate::filler(8),
                "mana": 10,
            },
            "p2": { "hand": hand, "library": crate::filler(8), "mana": 10 },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    fn forced(s: &Scenario) -> Vec<String> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AttackDeclared {
                    attacker_id,
                    forced: true,
                    ..
                } => Some(attacker_id.clone()),
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
        fn r1849006_no_switch_to_defense_and_every_unit_has_3_more_attack() {
            let mut s = played(false, json!([VANILLA]), json!([MOTHS, FILLER]));
            let mine = s.unit_at(P1, 1);
            s.expect_refused_with(|s| s.switch_position(&mine), NO_DEFENSE);
            assert_eq!(s.stats(mine.id.as_str()).attack, 4 + ATTACK);
            their_turn(&mut s);
            s.play(MOTHS, json!({ "zone": 3 }));
            s.expect_in_zone(MOTHS, "field");
            assert_eq!(s.stats(MOTHS).attack, 1 + ATTACK);
        }

        /// R783005: a Unit set face-down by its Cloaked is a Trap play, no Unit played.
        #[test]
        fn r783005_a_unit_set_face_down_by_its_cloaked_is_not_attacked() {
            let mut s = played(
                false,
                json!([{ "def": MOTHS, "lane": 1 }, { "def": MOTHS, "lane": 2 }, { "def": MOTHS, "lane": 3 }]),
                json!([crate::CLOAKED_UNIT, FILLER]),
            );
            their_turn(&mut s);
            crate::set_cloaked(&mut s, P2, crate::CLOAKED_UNIT, 1);
            assert!(forced(&s).is_empty(), "{:?}", forced(&s));
        }

        #[test]
        fn r1849007_each_of_its_units_attacks_the_arrival_in_lane_order() {
            let mut s = played(
                false,
                json!([{ "def": MOTHS, "lane": 1 }, { "def": MOTHS, "lane": 2 }, { "def": MOTHS, "lane": 3 }]),
                json!([GENN, FILLER]),
            );
            let mine = vec![s.unit_at(P1, 1).id, s.unit_at(P1, 2).id, s.unit_at(P1, 3).id];
            their_turn(&mut s);
            let cursor = s.state().rng_cursor;
            s.play(GENN, json!({ "zone": 1 }));
            assert_eq!(forced(&s), mine);
            s.expect_stats(GENN, json!({ "health": 14 - 3 * (1 + ATTACK) }));
            assert_eq!(s.state().rng_cursor, cursor, "no draw");
        }

        #[test]
        fn r1849007_the_run_stops_once_the_arrival_is_gone() {
            let mut s = played(
                false,
                json!([{ "def": VANILLA, "lane": 1 }, { "def": MOTHS, "lane": 2 }]),
                json!([VANILLA, FILLER]),
            );
            let first = s.unit_at(P1, 1).id;
            their_turn(&mut s);
            let theirs = s.hand(P2)[0].id.clone();
            s.play(theirs.as_str(), json!({ "zone": 1 }));
            assert_eq!(forced(&s), vec![first]);
        }

        #[test]
        fn r638_brittle_5_destroys_it_in_time() {
            let mut s = played(false, json!([]), json!([FILLER]));
            for _ in 0..4 {
                crate::next_turn_of(&mut s, P1);
            }
            assert_eq!(s.backrow(P1, 1).map(|card| card.def_id), Some(ID.to_string()));
            crate::next_turn_of(&mut s, P1);
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn attack_reads_through_param() {
            let mut s = played(false, json!([VANILLA]), json!([FILLER]));
            let disruptor = s.backrow(P1, 1).expect("played").id;
            assert_eq!(crate::degrade_number(&mut s, &disruptor, "attack"), ATTACK - 1);
            assert_eq!(s.stats(s.unit_at(P1, 1).id.as_str()).attack, 4 + ATTACK - 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1875003_its_controllers_units_strike_first() {
            let mut s = played(true, json!([GREASER]), json!([VANILLA, FILLER]));
            let mine = s.unit_at(P1, 1).id;
            assert!(unit_has(s.state(), s.card(mine.as_str()), KeywordKind::FirstStrike));
            their_turn(&mut s);
            s.play(VANILLA, json!({ "zone": 1 }));
            assert!(!unit_has(s.state(), s.card(VANILLA), KeywordKind::FirstStrike));
            s.expect_in_zone(VANILLA, "graveyard");
            s.expect_stats(mine.as_str(), json!({ "health": 7 }));
        }
    }

    /// R3800004: S #23's end of daily fuzz seed 9999's loop, on Warp Disruptor II's every-Unit answer.
    mod r3800004_it_answers_each_card_once_a_turn {
        use super::*;

        const JOB_APPLICATION: &str = "stinkyscape-112";

        #[test]
        fn r3800004_a_self_resummoning_unit_it_kills_is_answered_once_and_stays() {
            let mut s = crate::scenario(json!({
                "seed": "warp-disruptor-ii",
                "p1": {
                    "hand": [{ "def": ID, "radiant": true }, FILLER],
                    "field": [GREASER],
                    "library": crate::filler(8),
                    "mana": 10,
                },
                "p2": { "hand": [FILLER], "field": [VANILLA], "library": crate::filler(8), "mana": 10 },
            }));
            for player in [P1, P2] {
                s.state_mut().players[player].auto_end_turn = Some(false);
            }
            s.play(ID, json!({ "zone": 1 }));
            let mine = s.unit_at(P1, 1).id;
            let body = s.unit_at(P2, 1).id;
            crate::run_as(
                &mut s,
                jackioh_engine::effects::fuse_cards(json_as(json!({
                    "instanceIds": [body],
                    "defIds": [JOB_APPLICATION],
                    "targetInstanceId": body,
                }))),
                P2,
            );
            s.attack(mine.as_str(), body.as_str());
            let deaths = s
                .last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == body))
                .count();
            assert_eq!(deaths, 2, "it died to the attack and to the one answer");
            assert_eq!(forced(&s), vec![mine.clone()]);
            s.expect_in_zone(body.as_str(), "field");
            assert!(s.state().result.is_none());
        }
    }
}
