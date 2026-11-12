//! SPEC §8.12 S+ #26 Jlockeed's Forever War, a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    (1) Spell: "For the rest of the game, at the start of its owner's turn summon a Conscript."
//!   Radiant: "… summon a Radiant conscript."
//!
//! Engine: S+ #166's player effect for the rest of the game (R458): a `never`-expiry modifier on the
//! caster that acts at their start of turn among the start-of-turn triggers (R62), before the draw,
//! and never on the opponent's turns (R3549004, "its owner's"). Each turn it summons S+ #26.1
//! Conscript into the leftmost open unit zone (R64), failing silently on a full board, Radiant on the
//! Radiant face. The Spell goes to the graveyard and the effect is the player's, so no board wipe
//! ends it. `forRestOfGame` re-enters `delayed` with no `self` (R127). The badge is the face as the
//! catalog the game runs under prints it (`def_of`), so a replay of an older version badges it in
//! that version's words (R767000).

use jackioh_engine::effects::{for_rest_of_game, summon};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-026";

/// The rest-of-game effect's step (§10.6).
const STEP: &str = "conscripts";
/// The token each of its owner's turn starts summons (S+ #26.1 Conscript).
const CONSCRIPT: &str = "stinkyscapeplus-026-1";

fn war(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let face = if radiant { FaceKind::Radiant } else { FaceKind::Base };
            let label = fill_params(def_of(Some(&*ctx.state), ID), face, None);
            vec![for_rest_of_game(json_as(json!({ "step": STEP, "label": label })))]
        })),
        delayed: Some(hook(move |_ctx| {
            vec![summon(json_as(json!({ "defId": CONSCRIPT, "radiant": radiant })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: war(false),
        radiant: war(true),
    }
}

// S+ #26 Jlockeed's Forever War — SPEC §8.12 row S+ 26 (R458, R62, R64, R127, R3549004).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005"; // Stockpile: a Spell (§2.5).
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const NETHER: &str = "core-088"; // Twisting Nether: destroy all permanents

    fn holding(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(6), "mana": 10 },
            "p2": { "hand": [FILLER, NETHER], "library": filler(6), "mana": 10 },
        }))
    }

    /// Plays the Forever War in p1's hand.
    fn cast(s: &mut Scenario) {
        s.play(ID, json!({}));
    }

    fn next_turn(s: &mut Scenario) {
        s.end_turn().end_turn();
    }

    fn conscripts(s: &Scenario) -> Vec<CardInstance> {
        (1..=5)
            .filter_map(|lane| s.unit(P1, lane))
            .filter(|card| card.def_id == CONSCRIPT)
            .collect()
    }

    fn labels(s: &Scenario) -> Vec<String> {
        s.state()
            .players
            .p1
            .mods
            .iter()
            .filter_map(|m| match &m.kind {
                ModifierKind::StartOfTurnEffect { label, .. } => Some(label.clone()),
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r458_badge_is_the_face_text_and_spell_goes_to_graveyard() {
            let mut s = holding(false, "s26-badge");
            cast(&mut s);
            s.expect_in_zone(ID, "graveyard");
            let mods: Vec<&PlayerModifier> = s
                .state()
                .players
                .p1
                .mods
                .iter()
                .filter(|m| matches!(m.kind, ModifierKind::StartOfTurnEffect { .. }))
                .collect();
            assert_eq!(mods.len(), 1);
            assert_eq!(mods[0].expiry, ModifierExpiry::Never);
            // R169: its badge, public to both seats, in the card's own words.
            let label = fill_params(&crate::card_def(ID), FaceKind::Base, None);
            assert!(label.contains("summon a Conscript"), "{label}");
            assert_eq!(labels(&s), vec![label]);
        }

        #[test]
        fn r62_each_owner_turn_start_summons_before_the_draw() {
            let mut s = holding(false, "s26-cycles");
            cast(&mut s);
            for _turn in 0..3 {
                next_turn(&mut s);
                let fresh = conscripts(&s);
                assert_eq!(fresh.len(), _turn + 1);
                let events = s.last_events();
                let summoned = events
                    .iter()
                    .position(|event| matches!(event, GameEvent::Summoned { .. }))
                    .expect("a summon");
                let drawn = events
                    .iter()
                    .rposition(|event| matches!(event, GameEvent::Drawn { .. }))
                    .expect("the turn draw");
                assert!(summoned < drawn, "before the turn's draw");
            }
            // Base Conscripts: 1/1 Ranged, not Radiant.
            for conscript in conscripts(&s) {
                assert!(!conscript.radiant);
                s.expect_stats(conscript.id.as_str(), json!({ "attack": 1, "health": 1, "maxHealth": 1 }));
                assert!(unit_has(s.state(), &conscript, KeywordKind::Ranged));
            }
        }

        #[test]
        fn opponents_turns_summon_nothing() {
            let mut s = holding(false, "s26-opp");
            cast(&mut s);
            s.end_turn();
            assert_eq!(s.state().active, P2);
            assert!(conscripts(&s).is_empty());
        }

        #[test]
        fn full_board_summons_nothing() {
            let mut s = scenario(json!({
                "seed": "s26-full",
                "p1": {
                    "hand": [ID],
                    "field": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA],
                    "library": filler(6),
                    "mana": 10,
                },
                "p2": { "hand": [FILLER], "library": filler(6) },
            }));
            cast(&mut s);
            next_turn(&mut s);
            assert!(conscripts(&s).is_empty(), "a full row fails silently (R64)");
        }

        #[test]
        fn s10_1_destroying_every_permanent_does_not_end_it() {
            let mut s = holding(false, "s26-nether");
            cast(&mut s);
            s.end_turn();
            s.play(NETHER, json!({}));
            // Nether spends p2's last mana, so their turn may end by itself (§2.5).
            if s.state().active == P2 {
                s.end_turn();
            }
            assert_eq!(s.state().active, P1);
            assert_eq!(conscripts(&s).len(), 1, "the rest of the game outlives the board");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn radiant_summons_a_radiant_conscript() {
            let mut s = holding(true, "s26-radiant");
            cast(&mut s);
            let label = fill_params(&crate::card_def(ID), FaceKind::Radiant, None);
            assert!(label.contains("Radiant conscript"), "{label}");
            assert_eq!(labels(&s), vec![label]);
            next_turn(&mut s);
            let fresh = conscripts(&s);
            assert_eq!(fresh.len(), 1);
            assert!(fresh[0].radiant);
            s.expect_stats(fresh[0].id.as_str(), json!({ "attack": 2, "health": 2, "maxHealth": 2 }));
            assert!(unit_has(s.state(), &fresh[0], KeywordKind::Ranged));
            assert!(unit_has(s.state(), &fresh[0], KeywordKind::Rush));
            assert!(unit_has(s.state(), &fresh[0], KeywordKind::Pierce));
        }
    }
}
