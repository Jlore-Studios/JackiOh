//! S #154 Dice of Doom (SPEC §8.10 row S 154; §2.4, §6.1 Lucky, §6.3 Embiggen, R62, R153, R155, R386,
//! R640, R1178001, R1438, R1440, R1857002, R1870007): (0) Spell, Quickdraw, Mythic; Embiggened (1).
//!   Base:    "Roll a 100-sided die. If it lands on 67, add every Terminus card to your hand. They cost
//!            (0). / End of turn: Add this back to your hand. / Embiggened (1): This also gains Lucky +1
//!            for good."
//!   Radiant: "Roll an 80-sided die twice. If either lands on 67, …" and the same.
//!
//! Engine: a luck-based roll (§6.1) of 1 to `SIDES` from the match rng, a hit on `HIT`; the card's Lucky
//! (`lucky_on`, printed plus given) and its player's Luck roll it again, any `HIT` a hit (R1440). The
//! Radiant face rolls `RADIANT_ROLLS` dice of 1 to `RADIANT_SIDES`, each lucky the same way, either a
//! hit. Every roll is made, hit or not, so the draws a play takes never hang on the faces. Embiggened,
//! the instance first gains Lucky +1 for good (`gain_keyword_for_good`, R386, kept in every zone,
//! R1438), counting toward this play's roll. A hit adds each Terminus card (S #4's `TERMINUS_CARDS`,
//! R1857002) at `costOverride` 0. The dice are reported to both players as one batch before the Lucky
//! gain and the Terminus cards, every roll of each die with the one it kept (R2652001).

use jackioh_engine::effects::{add_to_hand, gain_keyword_for_good};
use jackioh_engine::prelude::*;

use crate::scripts::stinkyscape::c004_olivian_expanse::TERMINUS_CARDS;

pub const ID: &str = "stinkyscape-154";

/// The die: 1 to 100, a hit on 67; the Radiant face's two dice of 1 to 80.
const SIDES: i32 = 100;
const RADIANT_SIDES: i32 = 80;
const RADIANT_ROLLS: i32 = 2;
const HIT: i32 = 67;
/// "Embiggened (1): This also gains Lucky +1 for good."
const LUCKY_GAIN: i32 = 1;

fn roll(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let gain = if ctx.embiggened { LUCKY_GAIN } else { 0 };
    let lucky = ctx.live_self().map_or(0, |me| lucky_on(&*ctx.state, me)) + luck_of(&*ctx.state, ctx.controller) + gain;
    let (rolls, sides) = if ctx.radiant {
        (RADIANT_ROLLS, RADIANT_SIDES)
    } else {
        (1, SIDES)
    };
    let dice: Vec<RollResult> = (0..rolls)
        .map(|_| roll_die(ctx, sides, lucky, |face| face == HIT))
        .collect();
    let hit = dice.iter().any(|die| die.kept_face() == HIT);
    // R2652001: the dice are shown, every Lucky roll of each, before anything they decided.
    let mut effects = vec![report_rolls(RollKind::Die, sides, dice)];
    if gain > 0 {
        effects.push(gain_keyword_for_good(json_as(
            json!({ "keyword": { "kind": "Lucky", "n": gain } }),
        )));
    }
    if hit {
        effects.extend(
            TERMINUS_CARDS
                .iter()
                .map(|def_id| add_to_hand(json_as(json!({ "defId": def_id, "costOverride": 0 })))),
        );
    }
    effects
}

/// Core #31's return without the cost raise: from the graveyard, only on the turn it was played
/// (R153, R155). A full hand burns the bounce back (§2.4).
fn return_to_hand(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(self_) = ctx.live_self().cloned() else {
        return vec![];
    };
    if self_.zone.z() != ZoneName::Graveyard || !was_played_this_turn(ctx.state, self_.owner, &self_) {
        return vec![];
    }
    vec![bounce(json_as(json!({ "target": { "of": "self" } })))]
}

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(roll)),
        end_of_turn: Some(hook(return_to_hand)),
        ..Script::default()
    };
    // The face is read off the context, so both faces run the one script.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S #154 Dice of Doom — SPEC §8.10 row S 154, BUILD M12 row S 154 (R1870007).
#[cfg(test)]
mod tests {
    use super::{HIT, ID, RADIANT_SIDES};
    use crate::scripts::stinkyscape::c004_olivian_expanse::TERMINUS_CARDS;
    use jackioh_engine::rng::Rng;
    use jackioh_engine::testkit::*;
    use jackioh_engine::zones::move_to_zone;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-005"; // (1) Spell Stockpile.

    fn board_with(seed: &str, radiant: bool, fillers: usize) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend(std::iter::repeat_n(json!(FILLER), fillers));
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": [FILLER, FILLER, FILLER, FILLER], "mana": 10 },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER, FILLER] },
        }));
        s.state_mut().players.p1.auto_end_turn = Some(false);
        s.state_mut().players.p2.auto_end_turn = Some(false);
        s
    }

    fn board(seed: &str, radiant: bool) -> Scenario {
        board_with(seed, radiant, 1)
    }

    fn dice(s: &Scenario) -> CardInstance {
        s.state()
            .players
            .p1
            .hand
            .iter()
            .chain(&s.state().players.p1.graveyard)
            .find(|card| card.def_id == ID)
            .cloned()
            .expect("the dice are in hand or in the graveyard")
    }

    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1)
            .into_iter()
            .filter(|card| TERMINUS_CARDS.contains(&card.def_id.as_str()))
            .collect()
    }

    fn rolled(seed: &str, radiant: bool, play: Value) -> Scenario {
        let mut s = board(seed, radiant);
        s.play(ID, play);
        s
    }

    fn hits(seed: &str, radiant: bool) -> bool {
        !added(&rolled(seed, radiant, json!({}))).is_empty()
    }

    mod r1870007_dice_of_doom {
        use super::*;

        #[test]
        fn r1870007_rolls_once_and_a_67_adds_the_seven_at_zero() {
            let seed = seed_where("dice-of-doom-hit", |seed| hits(seed, false));
            let mut s = board(&seed, false);
            let cursor = s.state().rng_cursor;
            s.play(ID, json!({}));
            assert_eq!(s.state().rng_cursor, cursor + 1, "one roll, one draw");
            let defs: Vec<String> = added(&s).into_iter().map(|card| card.def_id).collect();
            assert_eq!(defs, TERMINUS_CARDS.map(str::to_string).to_vec(), "one of each, in order");
            for card in added(&s) {
                assert_eq!(card.cost_override, Some(0), "{} costs (0)", card.def_id);
                assert!(!card.radiant);
            }
            assert_eq!(Rng::new(&seed, cursor).int(100) + 1, HIT);
        }

        #[test]
        fn r1870007_a_miss_adds_nothing() {
            let seed = seed_where("dice-of-doom-miss", |seed| !hits(seed, false));
            let s = rolled(&seed, false, json!({}));
            assert!(added(&s).is_empty());
            assert_eq!(s.hand(P1).len(), 1, "the filler alone");
        }

        #[test]
        fn r1870007_a_full_hand_burns_the_rest() {
            let fillers = 5;
            let seed = seed_where("dice-of-doom-full", |seed| {
                let mut s = board_with(seed, false, fillers);
                s.play(ID, json!({}));
                !added(&s).is_empty()
            });
            let mut s = board_with(&seed, false, fillers);
            s.play(ID, json!({}));
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            let defs: Vec<String> = added(&s).into_iter().map(|card| card.def_id).collect();
            assert_eq!(defs, TERMINUS_CARDS[..HAND_CAP as usize - fillers].to_vec());
            let burned: Vec<String> = s
                .pile(P1, "graveyard")
                .into_iter()
                .map(|card| card.def_id)
                .filter(|def_id| TERMINUS_CARDS.contains(&def_id.as_str()))
                .collect();
            assert_eq!(burned, TERMINUS_CARDS[HAND_CAP as usize - fillers..].to_vec(), "the rest burn");
        }

        #[test]
        fn r1870007_back_in_hand_at_end_of_turn() {
            let mut s = rolled("dice-of-doom-back", false, json!({}));
            let card = dice(&s);
            s.expect_in_zone(&card, "graveyard");
            s.end_turn();
            s.expect_in_zone(&card, "hand");
            assert_eq!(s.hand(P1).iter().filter(|held| held.def_id == ID).count(), 1);
        }

        #[test]
        fn r1870007_embiggened_costs_one_and_gains_lucky_kept_in_hand_deck_and_graveyard() {
            let mut s = board("dice-of-doom-embiggened", false);
            s.state_mut().players.p1.mana.current = 1;
            s.play(ID, json!({ "embiggen": true }));
            s.expect_mana(P1, 0);
            let card = dice(&s);
            s.expect_in_zone(&card, "graveyard");
            assert_eq!(lucky_on(s.state(), &card), 1, "in the graveyard");
            s.end_turn();
            let card = dice(&s);
            s.expect_in_zone(&card, "hand");
            assert_eq!(lucky_on(s.state(), &card), 1, "in the hand");
            let mut moved = card.clone();
            let _ = move_to_zone(s.state_mut(), &mut moved, OffFieldZone::Library, Default::default());
            let in_deck = s.card(card.id.as_str()).clone();
            s.expect_in_zone(&in_deck, "library");
            assert_eq!(lucky_on(s.state(), &in_deck), 1, "in the deck");
        }

        #[test]
        fn r1870007_embiggened_rolls_once_more() {
            let mut s = board("dice-of-doom-more", false);
            let cursor = s.state().rng_cursor;
            s.play(ID, json!({ "embiggen": true }));
            assert_eq!(s.state().rng_cursor, cursor + 2, "its new Lucky rolls again this play");
            s.end_turn();
            s.end_turn();
            s.state_mut().players.p1.mana.current = 10;
            let cursor = s.state().rng_cursor;
            s.play(ID, json!({ "embiggen": true }));
            assert_eq!(s.state().rng_cursor, cursor + 3, "Lucky 2: three rolls");
            assert_eq!(lucky_on(s.state(), &dice(&s)), 2, "every Embiggened play makes it luckier");
            s.end_turn();
            s.end_turn();
            let cursor = s.state().rng_cursor;
            s.play(ID, json!({}));
            assert_eq!(s.state().rng_cursor, cursor + 3);
            assert_eq!(lucky_on(s.state(), &dice(&s)), 2);
        }

        #[test]
        fn r1870007_quickdraw() {
            let scripts = super::super::script();
            for face in [&scripts.base, &scripts.radiant] {
                assert_eq!(face.static_flags.as_ref().and_then(|flags| flags.quickdraw), Some(true));
            }
            crate::register_all();
            assert!(crate::card_def(ID).tags.contains(&Tag::Quickdraw));
        }

        /// The last step's one `rolled` batch, as `(sides, rolls)`, checking it is the dice's and comes
        /// before every card the play added to the hand.
        fn report(s: &Scenario) -> (i32, Vec<RollResult>) {
            let at = |kind: GameEventType| s.last_events().iter().position(|event| event.event_type() == kind);
            let batches: Vec<&GameEvent> =
                s.last_events().iter().filter(|event| event.event_type() == GameEventType::Rolled).collect();
            assert_eq!(batches.len(), 1, "one batch");
            if let Some(added) = at(GameEventType::AddedToHand) {
                assert!(at(GameEventType::Rolled) < Some(added), "the dice show before the cards they won");
            }
            let GameEvent::Rolled { def_id, kind, sides, rolls, face_down, .. } = batches[0] else {
                unreachable!("filtered to rolled");
            };
            assert_eq!((def_id.as_str(), *kind, *face_down), (ID, RollKind::Die, None));
            (*sides, rolls.clone())
        }

        /// R2652001: the base face's one d100 is reported with its one face; a hit keeps the 67 and adds
        /// the Terminus cards, a miss keeps its face and adds nothing.
        #[test]
        fn r2652001_base_one_d100_reported_and_a_67_kept_on_a_hit() {
            let seed = seed_where("dice-of-doom-reported-hit", |seed| hits(seed, false));
            let s = rolled(&seed, false, json!({}));
            let (sides, rolls) = report(&s);
            assert_eq!(sides, 100);
            assert_eq!(rolls, vec![RollResult { faces: vec![HIT], kept: 0 }]);
            assert_eq!(added(&s).len(), TERMINUS_CARDS.len());

            let seed = seed_where("dice-of-doom-reported-miss", |seed| !hits(seed, false));
            let cursor = board(&seed, false).state().rng_cursor;
            let face = Rng::new(&seed, cursor).int(100) + 1;
            let s = rolled(&seed, false, json!({}));
            assert_eq!(report(&s), (100, vec![RollResult { faces: vec![face], kept: 0 }]));
            assert_ne!(face, HIT);
            assert!(added(&s).is_empty());
        }

        /// R2652001: the Radiant face's two d80s are one batch of two dice, each its own face in draw
        /// order.
        #[test]
        fn r2652001_radiant_two_d80s_are_one_batch() {
            let seed = "dice-of-doom-reported-radiant";
            let cursor = board(seed, true).state().rng_cursor;
            let mut rng = Rng::new(seed, cursor);
            let faces = [rng.int(RADIANT_SIDES) + 1, rng.int(RADIANT_SIDES) + 1];
            let s = rolled(seed, true, json!({}));
            assert_eq!(
                report(&s),
                (
                    RADIANT_SIDES,
                    faces.iter().map(|face| RollResult { faces: vec![*face], kept: 0 }).collect()
                )
            );
            assert_eq!(added(&s).is_empty(), !faces.contains(&HIT));
        }

        /// R1438, R2652001: Embiggened, the Lucky it gains rolls its die once more, so the die reports
        /// both faces in draw order, a 67 kept if either is one.
        #[test]
        fn r2652001_embiggened_each_die_reports_two_faces() {
            for radiant in [false, true] {
                let seed = format!("dice-of-doom-reported-embiggened-{radiant}");
                let cursor = board(&seed, radiant).state().rng_cursor;
                let sides = if radiant { RADIANT_SIDES } else { 100 };
                let mut rng = Rng::new(&seed, cursor);
                let s = rolled(&seed, radiant, json!({ "embiggen": true }));
                let (reported_sides, rolls) = report(&s);
                assert_eq!(reported_sides, sides);
                assert_eq!(rolls.len(), if radiant { 2 } else { 1 });
                for die in &rolls {
                    let faces = vec![rng.int(sides) + 1, rng.int(sides) + 1];
                    let kept = i32::from(faces[0] != HIT && faces[1] == HIT);
                    assert_eq!(die, &RollResult { faces, kept });
                }
                assert_eq!(s.state().rng_cursor, cursor + 2 * rolls.len() as u32);
            }
        }

        #[test]
        fn r1870007_radiant_two_rolls_of_80() {
            let seed = seed_where("dice-of-doom-radiant", |seed| {
                let cursor = board(seed, true).state().rng_cursor;
                let mut rng = Rng::new(seed, cursor);
                rng.int(RADIANT_SIDES) + 1 != HIT && rng.int(RADIANT_SIDES) + 1 == HIT
            });
            let mut s = board(&seed, true);
            let cursor = s.state().rng_cursor;
            s.play(ID, json!({}));
            assert_eq!(s.state().rng_cursor, cursor + 2, "two dice");
            assert_eq!(added(&s).len(), TERMINUS_CARDS.len(), "the second die's 67 hits");
            let seed = seed_where("dice-of-doom-radiant-miss", |seed| !hits(seed, true));
            assert!(added(&rolled(&seed, true, json!({}))).is_empty());
        }
    }
}
