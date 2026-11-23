//! SPEC §8.12 S+ #79 A Glimpse into the Endless Sea (4) Spell, Wincon, Terminus, Mythic, a card of
//! the Stinkyscape+ set (R1420, R3535000; R3569005, R3569006).
//!   Base:    "Exile your deck and every permanent. You are immune to fatigue damage. Start an {count}
//!            turn countdown. When it ends, win the game." (7; 8 through v0.6.7k, issue #4285)
//!   Radiant: "Exile your deck and every permanent. Your hero gains {armor} Armor. You are immune to
//!            fatigue damage. Start an {count} turn countdown. When it ends, win the game." (7, Armor 5)
//!
//! Engine: the whole library is exiled with no draw, so no fatigue (§2.4), as S+ #80 does. The
//! countdown is `delay` "in N turns" (R1181000): a badge on its owner's side counts down the turns they
//! start, and at the start of their `count`th turn from now, before the draw (§2.2), the delayed step
//! wins them the game (R850; a hero at 0 still loses first). Only the owner's turns tick it, one made
//! on the other player's turn counting from their next (R3569005). The card is a Wincon, so Bob's Plan
//! may pick it: its plan is Core #75 Infinite Reserves, which keeps the empty deck from costing its
//! owner the game before the countdown ends (R3569006; `c001_bobs_plan.rs`). The Radiant face's Armor is
//! `gain_hero_armor`, the declared `armor` (R386), kept for the rest of the game (R124). Issue #3951 took
//! the base face's countdown from 10 to 8, a number, and gave the Radiant face its Armor, which
//! `script_v0_6_4c` leaves out for the replays of v0.6.4c and before (R767000). Issue #4154 has both
//! faces exile every permanent after the deck (R4154200): each side's board, the caster's first, by
//! `exile_board` as S #80 The Last Voyage's, face-down and Dormant cards included and only a card no
//! effect reaches spared; through v0.6.6h it exiled the deck alone, which `script_v0_6_6h` keeps. Issue
//! #4353 makes both faces' caster immune to fatigue damage for the rest of the game (R4353020): a
//! `fatigueImmune` player modifier, expiry `never`, after the exiles and the Armor, so each later draw from
//! the empty deck is a fatigue draw, counted and announced, that deals their hero nothing; through v0.6.8b
//! it gave none, which `script_v0_6_8b` keeps.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-079";

/// The countdown's badge (R169).
const BADGE: &str = "A Glimpse into the Endless Sea: you win";
/// The delayed step, which `delayed` runs.
const STEP: &str = "win";

/// `armor`: whether the face gains its hero the declared `armor` (the Radiant face, since issue #3951);
/// `board`: whether it exiles every permanent after the deck (R4154200, since issue #4154); `fatigue`:
/// whether it makes its caster immune to fatigue damage for the rest of the game (R4353020, since issue
/// #4353).
fn glimpse(armor: bool, board: bool, fatigue: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut effects = vec![exile_bottom_of_library(json_as(json!({
                "count": zone_cards(&*ctx.state, ctx.controller, OffFieldZone::Library).len(),
            })))];
            if board {
                effects.push(exile_board(json_as(json!({}))));
                effects.push(exile_board(json_as(json!({ "player": "enemy" }))));
            }
            if armor {
                effects.push(gain_hero_armor(json_as(json!({ "amount": param(ctx, "armor") }))));
            }
            if fatigue {
                effects.push(add_player_modifier(json_as(json!({
                    "mod": { "expiry": { "until": "never" }, "kind": "fatigueImmune" },
                }))));
            }
            effects.push(delay(json_as(json!({
                "at": { "phase": "start", "player": "self" },
                "step": STEP,
                "turns": param(ctx, "count"),
                "label": BADGE,
            }))));
            effects
        })),
        delayed: Some(hook(|_ctx| vec![win_game(json_as(json!({})))])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: glimpse(false, true, true),
        radiant: glimpse(true, true, true),
    }
}

/// R767000: the scripts through v0.6.8b, which gave no immunity to fatigue damage (issue #4353).
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_8b() -> CardScripts {
    CardScripts {
        base: glimpse(false, true, false),
        radiant: glimpse(true, true, false),
    }
}

/// R767000: the scripts through v0.6.6h, which exiled the deck and no permanent (issue #4154).
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_6h() -> CardScripts {
    CardScripts {
        base: glimpse(false, false, false),
        radiant: glimpse(true, false, false),
    }
}

/// R767000: the scripts through v0.6.4c, whose Radiant face gained no Armor (issue #3951). `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_6_4c() -> CardScripts {
    CardScripts {
        base: glimpse(false, false, false),
        radiant: glimpse(false, false, false),
    }
}

// S+ #79 A Glimpse into the Endless Sea — SPEC §8.12 row 79 (R3569005, R3569006; R1181000, R850).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, next_turn_of, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    fn game(seed: &str, radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "library": filler(5),
                // The empty deck is fatigue each turn (§2.4): a hero that outlasts a dozen of them.
                "health": 200,
                "mana": 4,
            },
            "p2": { "library": filler(20), "health": 200 },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    /// Every badge on p1's side, as `viewer` reads them (R169).
    fn all_badges(s: &Scenario, viewer: PlayerId) -> Vec<String> {
        let view = s.view(viewer);
        let modifiers = if viewer == P1 {
            view.you.modifiers
        } else {
            view.opponent.modifiers
        };
        modifiers.iter().map(|modifier| modifier.label.clone()).collect()
    }

    /// The countdown badges on p1's side, as `viewer` reads them (R169).
    fn badges(s: &Scenario, viewer: PlayerId) -> Vec<String> {
        all_badges(s, viewer).into_iter().filter(|label| label.starts_with(BADGE)).collect()
    }

    /// R4353020: the badge of the immunity to fatigue damage (`view_for`'s words for it).
    const FATIGUE_BADGE: &str = "You are immune to fatigue damage";

    /// How many `fatigueImmune` modifiers `player` holds, each for the rest of the game.
    fn immunities(s: &Scenario, player: PlayerId) -> usize {
        s.state().players[player]
            .mods
            .iter()
            .filter(|modifier| modifier.kind == ModifierKind::FatigueImmune && modifier.expiry == ModifierExpiry::Never)
            .count()
    }

    /// How many of p1's own turns pass after the cast before the game is won, and the winner.
    fn turns_to_win(s: &mut Scenario, bound: usize) -> (usize, Option<Winner>) {
        for turns in 1..=bound {
            next_turn_of(s, P1);
            if let Some(result) = s.state().result {
                return (turns, Some(result.winner));
            }
        }
        (bound, None)
    }

    mod base {
        use super::*;

        #[test]
        fn r3569005_the_deck_is_exiled_and_the_owner_wins_when_it_ends() {
            let mut s = game("glimpse-base", false);
            let deck: Vec<String> = s.pile(P1, "library").iter().map(|card| card.id.clone()).collect();
            assert_eq!(deck.len(), 5);
            s.play(ID, json!({}));
            assert!(s.pile(P1, "library").is_empty(), "the deck is exiled");
            assert_eq!(s.pile(P1, "exile").len(), 5);
            assert!(s.pile(P2, "library").len() == 20, "only the owner's deck");
            // The countdown is 7 of p1's turns (8 through v0.6.7k, issue #4285; 10 until issue #3951); none of
            // the first six ends it.
            let (turns, winner) = turns_to_win(&mut s, 12);
            assert_eq!((turns, winner), (7, Some(Winner::P1)));
            assert_eq!(s.state().players[P1].hero.armor, 0, "the base face gives no Armor");
            let result = s.state().result.expect("the game is over");
            assert_eq!(result.reason, GameOverReason::WonByEffect);
        }

        #[test]
        fn r3569005_the_badge_counts_down() {
            let mut s = game("glimpse-badge", false);
            s.play(ID, json!({}));
            let at = |turns: i32| vec![format!("{BADGE} in {turns} turns")];
            assert_eq!(badges(&s, P1), at(7));
            assert_eq!(badges(&s, P2), at(7), "the other seat reads it too");
            // The opponent's turn is no tick of it; p1's own is.
            s.end_turn();
            assert_eq!(badges(&s, P1), at(7), "the other player's turn does not tick it");
            s.end_turn();
            assert_eq!(badges(&s, P1), at(6));
            assert_eq!(badges(&s, P2), at(6));
        }

        #[test]
        fn r3569005_an_empty_deck_has_nothing_to_exile_and_the_countdown_still_starts() {
            let mut s = game("glimpse-empty", false);
            s.state_mut().players[P1].library.clear();
            s.play(ID, json!({}));
            let (turns, winner) = turns_to_win(&mut s, 12);
            // With no deck every draw is a fatigue draw, which hits nothing since issue #4353 (R4353020).
            assert_eq!((turns, winner), (7, Some(Winner::P1)));
            assert_eq!(s.state().players[P1].hero.health, 200);
        }

        /// R767000: through v0.6.8b it gave no immunity, so the empty deck's fatigue hits (issue #4353).
        #[test]
        fn r767000_the_script_kept_for_v0_6_8b_leaves_fatigue_hitting() {
            crate::under_kept(ID, script_v0_6_8b(), || {
                let mut s = game("glimpse-fatigue-kept", false);
                s.play(ID, json!({}));
                assert_eq!(immunities(&s, P1), 0);
                let health = s.state().players[P1].hero.health;
                next_turn_of(&mut s, P1);
                assert_eq!(s.state().players[P1].fatigue_count, 1);
                assert_eq!(s.state().players[P1].hero.health, health - FATIGUE_DAMAGE(1));
            });
        }

        /// R767000: through v0.6.4c neither face gave Armor (issue #3951).
        #[test]
        fn r767000_the_script_kept_for_v0_6_4c_gives_the_radiant_face_no_armor() {
            crate::under_kept(ID, script_v0_6_4c(), || {
                let mut s = game("glimpse-kept", true);
                s.play(ID, json!({}));
                assert_eq!(s.state().players[P1].hero.armor, 0);
            });
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3569005_radiant_counts_7_and_gives_your_hero_armor_5() {
            let mut s = game("glimpse-radiant", true);
            s.play(ID, json!({}));
            assert!(s.pile(P1, "library").is_empty());
            assert_eq!(s.state().players[P1].hero.armor, 5);
            let (turns, winner) = turns_to_win(&mut s, 12);
            assert_eq!((turns, winner), (7, Some(Winner::P1)));
        }
    }

    /// R4353020: both faces make their caster immune to fatigue damage for the rest of the game, issue
    /// #4353: each of their turns' draws from the exiled deck is a fatigue draw, counted, and their hero
    /// takes nothing. The badge says so on both seats; the opponent gets no immunity.
    #[test]
    fn r4353020_both_faces_make_the_caster_immune_to_fatigue_damage_for_the_rest_of_the_game() {
        for radiant in [false, true] {
            let mut s = game(&format!("glimpse-fatigue-{radiant}"), radiant);
            s.play(ID, json!({}));
            assert_eq!(immunities(&s, P1), 1, "radiant {radiant}");
            assert_eq!(immunities(&s, P2), 0, "radiant {radiant}");
            assert!(all_badges(&s, P1).contains(&FATIGUE_BADGE.to_string()), "radiant {radiant}");
            assert!(all_badges(&s, P2).contains(&FATIGUE_BADGE.to_string()), "radiant {radiant}");
            let health = s.state().players[P1].hero.health;
            for _turn in 0..3 {
                next_turn_of(&mut s, P1);
            }
            assert_eq!(s.state().players[P1].fatigue_count, 3, "radiant {radiant}");
            assert_eq!(s.state().players[P1].hero.health, health, "radiant {radiant}");
            assert_eq!(immunities(&s, P1), 1, "radiant {radiant}: it lasts");
        }
    }

    /// R4154200: "every permanent", each side's, since issue #4154.
    mod every_permanent {
        use super::*;

        const UNIT: &str = "stand-in (1) 4/4 R12/12";
        const TRAP: &str = "stand-in (1) Trap";

        /// The countdown's game, with p1's Unit and face-down Trap, and p2's Unit, Dormant Unit and
        /// face-down Trap: the instance ids of the five, p1's first.
        fn boards(seed: &str, radiant: bool) -> (Scenario, Vec<String>) {
            let mut s = scenario(json!({
                "seed": seed,
                "p1": {
                    "hand": [{ "def": ID, "radiant": radiant }],
                    "library": filler(5),
                    "field": [UNIT],
                    "backrow": [{ "def": TRAP, "lane": 2 }],
                    "health": 200,
                    "mana": 4,
                },
                "p2": {
                    "library": filler(20),
                    "health": 200,
                    "field": [UNIT, { "def": UNIT, "lane": 2, "slumber": { "turns": 2 } }],
                    "backrow": [{ "def": TRAP, "lane": 3 }],
                },
            }));
            for player in [P1, P2] {
                s.state_mut().players[player].auto_end_turn = Some(false);
            }
            let permanents = vec![
                s.unit(P1, 1).expect("p1's Unit").id,
                s.backrow(P1, 2).expect("p1's face-down Trap").id,
                s.unit(P2, 1).expect("p2's Unit").id,
                s.unit(P2, 2).expect("p2's Dormant Unit").id,
                s.backrow(P2, 3).expect("p2's face-down Trap").id,
            ];
            (s, permanents)
        }

        fn exiled(s: &Scenario, player: PlayerId) -> Vec<String> {
            s.pile(player, "exile").iter().map(|card| card.id.clone()).collect()
        }

        #[test]
        fn r4154200_both_faces_exile_every_permanent_face_down_and_dormant_ones_included() {
            for radiant in [false, true] {
                let (mut s, permanents) = boards("glimpse-board", radiant);
                s.play(ID, json!({}));
                assert!(s.pile(P1, "library").is_empty(), "the deck still goes");
                assert_eq!(s.pile(P2, "library").len(), 20, "the opponent's deck stays");
                let (mine, theirs) = (exiled(&s, P1), exiled(&s, P2));
                for id in &permanents[..2] {
                    assert!(mine.contains(id), "{id} reaches p1's exile (radiant {radiant})");
                }
                for id in &permanents[2..] {
                    assert!(theirs.contains(id), "{id} reaches p2's exile (radiant {radiant})");
                }
                for lane in 1..=5 {
                    assert!(s.unit(P1, lane).is_none() && s.unit(P2, lane).is_none(), "no Unit stays");
                    assert!(s.backrow(P1, lane).is_none() && s.backrow(P2, lane).is_none(), "nor a backrow card");
                }
                // The countdown and the Radiant face's Armor are as before.
                let armor = if radiant { 5 } else { 0 };
                assert_eq!(s.state().players[P1].hero.armor, armor);
                let (turns, winner) = turns_to_win(&mut s, 12);
                assert_eq!((turns, winner), (7, Some(Winner::P1)));
            }
        }

        /// R767000: through v0.6.6h the deck alone was exiled (issue #4154).
        #[test]
        fn r767000_the_script_kept_for_v0_6_6h_leaves_the_board() {
            crate::under_kept(ID, script_v0_6_6h(), || {
                for radiant in [false, true] {
                    let (mut s, permanents) = boards("glimpse-board-kept", radiant);
                    s.play(ID, json!({}));
                    assert!(s.pile(P1, "library").is_empty());
                    assert_eq!(s.pile(P1, "exile").len(), 5, "the deck alone");
                    assert!(s.pile(P2, "exile").is_empty());
                    assert_eq!(s.unit(P1, 1).map(|card| card.id), Some(permanents[0].clone()));
                    assert_eq!(s.unit(P2, 2).map(|card| card.id), Some(permanents[3].clone()));
                    let armor = if radiant { 5 } else { 0 };
                    assert_eq!(s.state().players[P1].hero.armor, armor);
                }
            });
        }
    }
}
