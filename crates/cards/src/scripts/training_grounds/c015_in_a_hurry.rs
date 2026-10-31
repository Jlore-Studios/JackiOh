//! TG #15 In a hurry (SPEC §8.11 row TG 15; §2.2, §3.2, §4.2, §7, R11, R64, R174, R350, R2198002): (2)
//! Spell, Common, a card of the held Training Grounds set (R1420, R2194000).
//!   Base:    "Summon {tokens|Rush Token|Rush Tokens}. At the end of this turn, destroy them." — 3.
//!   Radiant: "Summon {tokens|Radiant Rush Token|Radiant Rush Tokens}. …" — 3.
//! C+ #15 summon leftmost open zones (R64; fewer if full, §3.2; rememberAs, R1126; §4.2, §7).
//! End-of-turn delay destroys at §2.2 (R2198002, R350; off field forgotten, R11, R174; stolen still destroyed; END_OF_TURN_MARK, R437, R2321001; TG6 row 10).

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-015";

/// The token summoned, C+ #15's.
const RUSH_TOKEN: &str = "core-t-rush";
/// The step the end-of-turn destroy is stored under (the engine's hook resolves it by its data).
const DESTROY_STEP: &str = "destroy";

/// The key the `n`th token is remembered under.
fn token_key(n: i32) -> String {
    format!("token{n}")
}

/// The tokens this cast summoned that are still on the field, by id.
fn summoned(ctx: &mut EffectContext<'_>, tokens: i32) -> Vec<String> {
    (0..tokens)
        .filter_map(|n| recalled(ctx, &token_key(n)))
        .filter_map(|id| id.as_str().map(str::to_string))
        .collect()
}

fn hurry() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let tokens = param(&*ctx, "tokens");
            let radiant = ctx.radiant;
            let mut effects: Vec<Effect> = (0..tokens)
                .map(|n| {
                    summon(json_as(json!({
                        "defId": RUSH_TOKEN,
                        "radiant": radiant,
                        "rememberAs": token_key(n),
                    })))
                })
                .collect();
            effects.push(for_each_card(ForEachCardArgs {
                cards: Arc::new(move |ctx| summoned(ctx, tokens)),
                each: Arc::new(|id| {
                    // R2198002, R350: the end of this turn, whoever's.
                    delay(json_as(json!({
                        "at": { "phase": "end", "player": THIS_TURN },
                        "step": DESTROY_STEP,
                        "hook": DELAYED_DESTROY_HOOK,
                        "data": { "instanceId": id },
                        "watch": id,
                        // R2321001: the token wears it while the destroy waits.
                        "mark": { "mark": END_OF_TURN_MARK, "color": END_OF_TURN_MARK_COLOR },
                    })))
                }),
            }));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: hurry(),
        radiant: hurry(),
    }
}

// TG #15 In a hurry — SPEC §8.11 row TG 15, BUILD M13 row TG 15.
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::{END_OF_TURN_MARK, END_OF_TURN_MARK_COLOR};
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario, upgrade_number};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const HURRY: &str = "traininggrounds-015";
    const RUSH_TOKEN: &str = "core-t-rush";
    const GREASER: &str = "meditative-100"; // (2) Unit 7/7.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const ROCK: &str = "core-066"; // (4) Unit 10/10, Indestructible.
    const SUPPORT_LOSER: &str = "classicplus-019-4"; // End of turn: heal your hero and each of your Units 3.
    const ACE: &str = "classic-065"; // Trap; Radiant: at the end of your turn, Recruit 1 or 3.

    fn board(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": HURRY, "radiant": radiant }], "field": mine, "mana": 10, "library": filler(4) },
            "p2": { "hand": filler(2), "field": theirs, "library": filler(4) },
        }))
    }

    /// The Rush Tokens on `player`'s side, by lane.
    fn tokens(s: &Scenario, player: PlayerId) -> Vec<(i32, CardInstance)> {
        (1..=5)
            .filter_map(|lane| s.unit(player, lane).map(|card| (lane, card)))
            .filter(|(_, card)| card.def_id == RUSH_TOKEN)
            .collect()
    }

    fn lanes(s: &Scenario, player: PlayerId) -> Vec<i32> {
        tokens(s, player).into_iter().map(|(lane, _)| lane).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn three_rush_tokens_in_the_leftmost_open_unit_zones() {
            let mut s = board(false, json!([{ "def": VANILLA, "lane": 2 }]), json!([]));
            s.play(HURRY, json!({}));
            assert_eq!(lanes(&s, P1), vec![1, 3, 4]);
            for (_, token) in tokens(&s, P1) {
                s.expect_stats(&token, json!({ "attack": 3, "health": 3 }));
                assert!(!token.radiant);
            }
        }

        #[test]
        fn fewer_on_a_nearly_full_board_none_on_a_full_one() {
            let mut s = board(false, json!([VANILLA, VANILLA, VANILLA, VANILLA]), json!([]));
            s.play(HURRY, json!({}));
            assert_eq!(lanes(&s, P1), vec![5]);

            let mut s = board(false, json!([VANILLA, VANILLA, VANILLA, VANILLA, VANILLA]), json!([]));
            s.play(HURRY, json!({}));
            assert!(tokens(&s, P1).is_empty());
            s.end_turn();
            assert_eq!(
                (1..=5).filter(|lane| s.unit(P1, *lane).is_some()).count(),
                5,
                "nothing else is destroyed"
            );
        }

        #[test]
        fn they_may_attack_units_this_turn_and_not_the_hero() {
            let mut s = board(false, json!([]), json!([GREASER]));
            s.play(HURRY, json!({}));
            let token = tokens(&s, P1)[0].1.clone();
            let enemy = s.unit_at(P2, 1);
            let attacks: Vec<String> = legal_actions(s.state(), P1)
                .into_iter()
                .filter_map(|action| match action {
                    ActionBody::Attack { attacker_id, target_id } if attacker_id == token.id => Some(target_id),
                    _ => None,
                })
                .collect();
            assert!(attacks.contains(&enemy.id));
            assert!(!attacks.contains(&"hero-p2".to_string()));
        }

        #[test]
        fn r2198002_at_the_end_of_this_turn_each_still_on_the_field_is_destroyed() {
            let mut s = board(false, json!([]), json!([]));
            s.play(HURRY, json!({}));
            let made: Vec<CardInstance> = tokens(&s, P1).into_iter().map(|(_, card)| card).collect();
            assert_eq!(made.len(), 3);
            s.end_turn();
            assert!(tokens(&s, P1).is_empty());
            let destroyed: Vec<String> = s
                .last_events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Destroyed { instance_id, .. } => Some(instance_id.clone()),
                    _ => None,
                })
                .collect();
            for token in &made {
                assert!(destroyed.contains(&token.id), "a destroy, not a bounce or an exile");
            }
        }

        #[test]
        fn none_stands_on_the_opponents_turn() {
            let mut s = board(false, json!([]), json!([]));
            s.play(HURRY, json!({}));
            s.end_turn();
            assert_eq!(s.state().active, P2);
            assert!(tokens(&s, P1).is_empty());
        }

        #[test]
        fn one_already_dead_is_not_counted_again_and_nothing_else_is_destroyed() {
            let mut s = board(false, json!([{ "def": ROCK, "lane": 5 }]), json!([GREASER]));
            let rock = s.unit_at(P1, 5);
            let greaser = s.unit_at(P2, 1);
            s.play(HURRY, json!({}));
            let first = tokens(&s, P1)[0].1.clone();
            s.attack(&first, &greaser);
            s.expect_in_zone(&first, "gone");
            assert_eq!(tokens(&s, P1).len(), 2);
            s.end_turn();
            assert!(tokens(&s, P1).is_empty());
            s.expect_in_zone(&rock, "field");
            s.expect_in_zone(&greaser, "field");
        }

        #[test]
        fn r2198002_they_are_destroyed_after_the_end_of_turn_triggers_and_traps() {
            // p1's Support Loser (lane 5) heals each of p1's Units at the end of the turn, and p1's Radiant
            // Ace in the Hole Recruits in the end-of-turn trap window whatever its coin (R62). Both must
            // still find the tokens standing in lanes 1 to 3: the heal reaches the damaged one, and the
            // recruited Vanilla lands in lane 4, the only open zone, not in lane 1.
            let mut s = scenario(json!({
                "p1": {
                    "hand": [HURRY],
                    "field": [{ "def": SUPPORT_LOSER, "lane": 5 }],
                    "backrow": [{ "def": ACE, "radiant": true, "faceUp": false }],
                    "mana": 10,
                    "library": [VANILLA, VANILLA, VANILLA, VANILLA],
                },
                "p2": { "hand": filler(2), "library": filler(4) },
            }));
            s.play(HURRY, json!({}));
            assert_eq!(lanes(&s, P1), vec![1, 2, 3]);
            let first = tokens(&s, P1)[0].1.clone();
            s.card_mut(&first).damage = 1;
            s.end_turn();

            let events = s.last_events().to_vec();
            let healed = events.iter().position(
                |event| matches!(event, GameEvent::Healed { target_id, .. } if *target_id == first.id),
            );
            let recruited = events.iter().position(|event| {
                matches!(event, GameEvent::Summoned { def_id, .. } if def_id == VANILLA)
            });
            let destroyed = events.iter().position(
                |event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == first.id),
            );
            let (Some(healed), Some(recruited), Some(destroyed)) = (healed, recruited, destroyed) else {
                panic!("a heal, a recruit and a destroy: {healed:?} {recruited:?} {destroyed:?}");
            };
            assert!(healed < destroyed && recruited < destroyed);
            assert!(tokens(&s, P1).is_empty());
            assert_eq!(s.unit(P1, 4).map(|card| card.def_id).as_deref(), Some(VANILLA));
            assert!((1..=3).all(|lane| s.unit(P1, lane).is_none()));
            assert_eq!(s.unit(P1, 5).map(|card| card.def_id).as_deref(), Some(SUPPORT_LOSER));
        }

        #[test]
        fn r2198002_cast_on_the_opponents_turn_they_go_at_that_turns_end() {
            // p2 is active; p1 casts In a hurry (a free cast by p1's effect, R70), so the tokens are p1's
            // and arrive on p2's turn. They go at the end of p2's turn, before p1's turn begins.
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": filler(1), "library": filler(4) },
                "p2": { "hand": filler(2), "library": filler(4) },
            }));
            crate::run_as(&mut s, jackioh_engine::effects::cast_new(json_as(json!({ "def": HURRY }))), P1);
            assert_eq!(s.state().active, P2);
            assert_eq!(tokens(&s, P1).len(), 3);
            s.end_turn();
            assert_eq!(s.state().active, P1);
            assert!(tokens(&s, P1).is_empty());
        }

        /// What `viewer` reads on `id`'s view: the marks it carries, by name.
        fn marks_seen(s: &Scenario, viewer: PlayerId, id: &str) -> Vec<String> {
            let view = s.view(viewer);
            view.you
                .units
                .iter()
                .chain(view.opponent.units.iter())
                .flatten()
                .find(|unit| unit.instance_id == id)
                .and_then(|unit| unit.marks.clone())
                .unwrap_or_default()
                .into_iter()
                .map(|mark| mark.mark)
                .collect()
        }

        #[test]
        fn r2321001_each_token_wears_the_end_of_turn_mark_in_both_views_until_it_goes() {
            let mut s = board(false, json!([{ "def": VANILLA, "lane": 4 }]), json!([]));
            let vanilla = s.unit_at(P1, 4);
            s.play(HURRY, json!({}));
            let made: Vec<CardInstance> = tokens(&s, P1).into_iter().map(|(_, card)| card).collect();
            for token in &made {
                assert_eq!(marks_seen(&s, P1, &token.id), vec![END_OF_TURN_MARK.to_string()]);
                assert_eq!(marks_seen(&s, P2, &token.id), vec![END_OF_TURN_MARK.to_string()]);
            }
            assert!(
                s.last_events().iter().any(|event| matches!(
                    event,
                    GameEvent::Marked { instance_id, mark, color, added: true }
                        if *instance_id == made[0].id && mark == END_OF_TURN_MARK && color == END_OF_TURN_MARK_COLOR
                )),
                "the mark is announced as it lands"
            );
            assert!(marks_seen(&s, P1, &vanilla.id).is_empty(), "nothing else is marked");
            s.end_turn();
            assert!(tokens(&s, P1).is_empty());
            assert!(s.state().marks.clone().unwrap_or_default().is_empty(), "the marks went with the tokens");
        }

        #[test]
        fn r2321001_a_token_that_died_first_takes_its_mark_with_it() {
            let mut s = board(false, json!([]), json!([GREASER]));
            let greaser = s.unit_at(P2, 1);
            s.play(HURRY, json!({}));
            let first = tokens(&s, P1)[0].1.clone();
            s.attack(&first, &greaser);
            s.expect_in_zone(&first, "gone");
            assert!(
                !marks_on(s.state(), &first.id).iter().any(|mark| mark.mark == END_OF_TURN_MARK),
                "R174: its watch is forgotten, and its mark with it"
            );
            assert_eq!(
                s.state().marks.clone().unwrap_or_default().len(),
                2,
                "the two left standing still wear theirs"
            );
        }

        #[test]
        fn tokens_reads_through_param() {
            let mut s = board(false, json!([]), json!([]));
            assert_eq!(upgrade_number(&mut s, HURRY, "tokens"), 4);
            s.play(HURRY, json!({}));
            assert_eq!(tokens(&s, P1).len(), 4);
            s.end_turn();
            assert!(tokens(&s, P1).is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn three_radiant_rush_tokens_six_six_rush_and_cleave() {
            let mut s = board(true, json!([]), json!([]));
            s.play(HURRY, json!({}));
            let made = tokens(&s, P1);
            assert_eq!(made.len(), 3);
            for (_, token) in made {
                assert!(token.radiant);
                s.expect_stats(&token, json!({ "attack": 6, "health": 6 }));
                let kinds: Vec<KeywordKind> = s.stats(&token).keywords.iter().map(Keyword::kind).collect();
                assert!(kinds.contains(&KeywordKind::Rush) && kinds.contains(&KeywordKind::Cleave));
            }
        }

        #[test]
        fn destroyed_the_same_way() {
            let mut s = board(true, json!([]), json!([]));
            s.play(HURRY, json!({}));
            s.end_turn();
            assert!(tokens(&s, P1).is_empty());
        }
    }
}
