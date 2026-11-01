//! S #23 Warp Disruptor (SPEC §8.10 row 23; R1849006, R1849007, R1875003): (1) Field Spell, Rare, of
//! the Stinkyscape set.
//!   Base:    "Stack, Brittle 3 / Aura: Units can't be in Defense Position and have +{attack} Attack. /
//!            After your opponent summons a Unit, a random Unit of yours attacks it."
//!   Radiant: the same, and "Your Units have First Strike."
//! Engine: Stack and Brittle 3 are catalog keywords (§6.1, R638). While it acts face-up on the field:
//! `staticFlags.unitsNeverDefense` keeps every Unit of both sides out of Defense Position, one in it
//! switching to Attack at the state check, spending nothing (R1849006); an aura (§10.4) gives every Unit
//! of both sides +{attack} Attack (R1875003). After a Unit arrives on the opponent's side, however
//! summoned — a played Unit once it resolves, its Cry first, as C #5 Tesla reads it (R1875003) — one of
//! its controller's Units makes a forced attack on it, drawn from the match rng among those that may
//! attack it, none able drawing nothing (`forced_attacks` with `random`, R1849007, R53). Radiant: its
//! controller's Units have First Strike too (an aura). Tunes: attack 2 ↑.

use jackioh_engine::effects::forced_attacks;
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
            permanent,
            def_id,
            instance_id,
            ..
        } => {
            if !*permanent || def_of(Some(state), def_id).type_ != CardType::Unit {
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

fn disruptor(radiant: bool) -> Script {
    let answer = TriggerDef::new(
        "warp-disruptor",
        &[GameEventType::Summoned, GameEventType::CardResolved],
        |ctx, event| {
            let Some(target) = arrival(ctx, event) else {
                return Vec::new();
            };
            vec![forced_attacks(json_as(json!({
                "attackers": { "side": "self", "random": true },
                "target": { "instanceId": target },
            })))]
        },
    )
    .with_when(|ctx, event| arrival(ctx, event).is_some());
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

pub fn script() -> CardScripts {
    CardScripts {
        base: disruptor(false),
        radiant: disruptor(true),
    }
}

// S #23 Warp Disruptor — SPEC §8.10 row 23, BUILD M12 row S 23: "while it stands no Unit of either side
// can switch to Defense, and one already in it switches to Attack at the next state check; every Unit
// has +2 Attack; after the opponent plays or summons a Unit, one random Unit of its controller's that can
// attack it does so, a Flying arrival drawing only a Flying or Ranged attacker; with none able nothing
// happens and the rng does not move; Brittle 3 destroys it in time (R638); attack reads through
// `param()`; radiant its controller's Units have First Strike too".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;

    const FILLER: &str = "core-005"; // (1) Spell Stockpile: a card in hand, so no turn ends by itself.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const MOTHS: &str = "core-009"; // (2) 1/14.
    const STRATOS: &str = "stinkyscape-068"; // (1) 3/1 Flying, Ranged.
    const GREASER: &str = "meditative-100"; // (2) 7/7.
    const ATTACK: i32 = 2;
    /// "that unit cannot be in Defense Position" (R1849006).
    const NO_DEFENSE: &str = "cannot be in Defense Position";

    /// p1, active with 10 mana, plays Warp Disruptor (`radiant` its face) into backrow zone 1 with
    /// `mine` on its unit zones; p2 has `theirs` and `hand`. No turn ends by itself.
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

    /// The forced attacks among the last events, each `(attacker, target)`.
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

    /// p2's turn, with 10 mana.
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
            // The draw follows the play's resolution: its `cardResolved` comes before the attack.
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
            // 9 First Strike damage kills the 6/4 before it strikes back.
            s.expect_in_zone(VANILLA, "graveyard");
            s.expect_stats(mine.as_str(), json!({ "health": 7 }));
        }
    }
}
