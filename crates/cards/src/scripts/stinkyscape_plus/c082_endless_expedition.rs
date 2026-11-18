//! SPEC §8.12 S+ #82 Endless Expedition (1) Spell, Quickdraw, Mythic, a card of the Stinkyscape+ set
//! (R1420, R3535000; R3569007, R3569008, R4154101, R640, R458).
//!   Base:    "For the rest of the game, you have ({mana}) less mana, and at the start of each of your
//!            turns, add a random Terminus card to your hand. It costs (3)." — mana 1
//!   Radiant: "For the rest of the game, you have ({mana}) less mana, and at the start of each of your
//!            turns, add a random Radiant Terminus card to your hand. It costs (3)." — mana 1
//!
//! Engine (R4154101): the Cry lowers its caster's max mana by the declared `mana` for the rest of the
//! game (`loseMaxMana`, R3569008) and leaves `forRestOfGame` (C+ #52's, R458), a badge in the card's
//! words (R169) whose step adds a card at the start of each of its caster's turns from the next on, in
//! R62's delayed stage, before the draw; none comes now. Each card is one drawn from the Terminus pool,
//! every Terminus card of the shipped sets, tokens included (R3552001, R380), through the generator
//! (R387), costing (`TERMINUS_COST`) whatever it printed (R3569007), a Radiant one on the Radiant face.
//! Quickdraw (R640) is a static flag beside the tag.
//!
//! Kept for replays (R767000): through v0.6.6h the card read "Now and at the start of each of your turns
//! for the rest of the game: Add a random Terminus card to your hand. It costs (2).", with no mana lost
//! (`script_v0_6_6h`, issue #4154); through v0.6.5b it read "For the rest of the game, lose {mana} mana.
//! Add a random Terminus card to your hand. It costs (2).", `loseMaxMana` lowering its caster's max mana
//! (R3569008) and one card added (`script_v0_6_5b`, issue #3996); through v0.6.4c that card cost (3)
//! (`script_v0_6_4c`, issue #3951).

use jackioh_engine::effects::for_rest_of_game;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-082";

/// "It costs (3)": the added card's cost, since issue #4154.
const TERMINUS_COST: i32 = 3;
/// After v0.6.4c through v0.6.6h (issues #3951 to #4154): "It costs (2)".
const TERMINUS_COST_V0_6_6H: i32 = 2;
/// Through v0.6.4c (issue #3951): "it costs (3)".
const TERMINUS_COST_V0_6_4C: i32 = 3;
/// The step the start-of-turn add is re-entered under (`delayed`).
const EXPEDITION_STEP: &str = "expedition";

/// R640: Quickdraw, on every version.
fn quickdraw() -> Option<StaticFlags> {
    Some(StaticFlags {
        quickdraw: Some(true),
        ..StaticFlags::default()
    })
}

/// "Add a random Terminus card to your hand. It costs (`cost`)." (R3569007), Radiant on the Radiant face.
fn add_terminus(radiant: bool, cost: i32) -> Effect {
    add_random_from_catalog(json_as(json!({
        "query": crate::query::terminus_cards(),
        "radiant": radiant,
        "costOverride": cost,
    })))
}

/// R4154101: the caster's max mana lowered by the declared `mana` for the rest of the game (R3569008),
/// and the rest-of-game step that adds a card costing (3) at each of the caster's turn starts; none now.
fn expedition(radiant: bool) -> Script {
    Script {
        static_flags: quickdraw(),
        cry: Some(hook(move |ctx| {
            let face = if radiant { FaceKind::Radiant } else { FaceKind::Base };
            let label = fill_params(def_of(Some(&*ctx.state), ID), face, None);
            vec![
                lose_max_mana(json_as(json!({ "amount": param(ctx, "mana") }))),
                for_rest_of_game(json_as(json!({ "step": EXPEDITION_STEP, "label": label }))),
            ]
        })),
        delayed: Some(hook(move |_ctx| vec![add_terminus(radiant, TERMINUS_COST)])),
        ..Script::default()
    }
}

/// Issue #3996, through v0.6.6h: the card now, and the rest-of-game step that adds one at each of the
/// caster's turn starts, each costing (2).
fn expedition_v0_6_6h(radiant: bool) -> Script {
    Script {
        static_flags: quickdraw(),
        cry: Some(hook(move |ctx| {
            let face = if radiant { FaceKind::Radiant } else { FaceKind::Base };
            let label = fill_params(def_of(Some(&*ctx.state), ID), face, None);
            vec![
                add_terminus(radiant, TERMINUS_COST_V0_6_6H),
                for_rest_of_game(json_as(json!({ "step": EXPEDITION_STEP, "label": label }))),
            ]
        })),
        delayed: Some(hook(move |_ctx| vec![add_terminus(radiant, TERMINUS_COST_V0_6_6H)])),
        ..Script::default()
    }
}

/// Through v0.6.5b: the caster's max mana lowered by the declared `mana` for the rest of the game
/// (R3569008), and one card added costing `cost`.
fn expedition_v0_6_5b(radiant: bool, cost: i32) -> Script {
    Script {
        static_flags: quickdraw(),
        cry: Some(hook(move |ctx| {
            vec![
                lose_max_mana(json_as(json!({ "amount": param(ctx, "mana") }))),
                add_terminus(radiant, cost),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: expedition(false),
        radiant: expedition(true),
    }
}

/// R767000: the scripts after v0.6.5b through v0.6.6h: a Terminus card costing (2) now and at each of the
/// caster's turn starts, no mana lost (issue #4154). `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_6h() -> CardScripts {
    CardScripts {
        base: expedition_v0_6_6h(false),
        radiant: expedition_v0_6_6h(true),
    }
}

/// R767000: the scripts after v0.6.4c through v0.6.5b: max mana lowered for the rest of the game and one
/// Terminus card added, costing (2) (issue #3996). `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_5b() -> CardScripts {
    CardScripts {
        base: expedition_v0_6_5b(false, TERMINUS_COST_V0_6_6H),
        radiant: expedition_v0_6_5b(true, TERMINUS_COST_V0_6_6H),
    }
}

/// R767000: the scripts through v0.6.4c, whose Terminus card cost (3) (issue #3951). `KEPT_SCRIPTS` folds
/// those replays under it.
pub fn script_v0_6_4c() -> CardScripts {
    CardScripts {
        base: expedition_v0_6_5b(false, TERMINUS_COST_V0_6_4C),
        radiant: expedition_v0_6_5b(true, TERMINUS_COST_V0_6_4C),
    }
}

// S+ #82 Endless Expedition — SPEC §8.12 row 82 (R4154101, R3569007, R3569008; R640, R3552001, R387).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";

    fn game(seed: &str, radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "library": filler(10), "mana": 4 },
            "p2": { "library": filler(10) },
        }));
        // Four mana crystals: the fourth turn's, full and open.
        let side = &mut s.state_mut().players[P1];
        side.turns_started = 4;
        side.mana.max = 4;
        side.mana.current = 4;
        for player in [P1, PlayerId::P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    /// The Terminus pool of the shipped sets, tokens included.
    fn pool() -> Vec<String> {
        crate::register_all();
        crate::query::query(&crate::query::terminus_cards())
            .into_iter()
            .map(|def| def.id.clone())
            .collect()
    }

    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1)
            .into_iter()
            .filter(|card| card.def_id != FILLER)
            .collect()
    }

    /// Each card added is a Terminus card of the pool, Radiant on the Radiant face, costing `cost`.
    fn expect_terminus(s: &Scenario, made: &[CardInstance], radiant: bool, cost: i32) {
        let pool = pool();
        for card in nonempty(made.to_vec(), "the cards added") {
            assert!(pool.contains(&card.def_id), "{} is no Terminus card", card.def_id);
            assert_eq!(card.radiant, radiant, "{}", card.def_id);
            assert_eq!(effective_cost(s.state(), &card, Default::default()), cost, "{}", card.def_id);
        }
    }

    /// R767000: `run` under a kept script, with the `mana` the card declared through v0.6.5b (1, both faces).
    fn kept<T>(scripts: CardScripts, run: impl FnOnce() -> T) -> T {
        crate::under_kept_as(
            ID,
            scripts,
            |def| {
                def.params = serde_json::from_value(json!([
                    { "key": "mana", "base": 1, "radiant": 1, "better": "down", "step": 1, "min": 1 }
                ]))
                .expect("the v0.6.5b params");
            },
            run,
        )
    }

    mod base {
        use super::*;

        #[test]
        fn r640_it_is_quickdraw() {
            for scripts in [script(), script_v0_6_6h(), script_v0_6_5b(), script_v0_6_4c()] {
                assert_eq!(scripts.base.flags().quickdraw, Some(true));
                assert_eq!(scripts.radiant.flags().quickdraw, Some(true));
            }
        }

        /// R4154101, R3569008: issue #4154 brought back the max-mana loss and took out the card now.
        #[test]
        fn r4154101_it_lowers_max_mana_by_1_for_the_rest_of_the_game_and_adds_no_card_now() {
            let mut s = game("expedition-base", false);
            s.play(ID, json!({}));
            // Cast for (1) from 4: 3 left of a max that fell from 4 to 3.
            assert_eq!(s.state().players[P1].mana.max, 3);
            assert_eq!(s.state().players[P1].mana.current, 3);
            assert!(added(&s).is_empty(), "no card now");
            s.end_turn().end_turn();
            assert_eq!(s.state().active, P1);
            assert_eq!(s.state().players[P1].mana.max, 3, "the max stays lower");
        }

        #[test]
        fn r458_it_leaves_a_rest_of_game_modifier_badged_in_its_words_and_goes_to_the_graveyard() {
            let mut s = game("expedition-badge", false);
            s.play(ID, json!({}));
            s.expect_in_zone(ID, "graveyard");
            let label = fill_params(&crate::card_def(ID), FaceKind::Base, None);
            let mine: Vec<String> = s.view(P1).you.modifiers.iter().map(|m| m.label.clone()).collect();
            assert_eq!(mine, vec![label.clone()]);
            let theirs: Vec<String> =
                s.view(PlayerId::P2).opponent.modifiers.iter().map(|m| m.label.clone()).collect();
            assert_eq!(theirs, vec![label]);
        }

        /// R458, R62, R3569007, R4154101: one at the start of each of the caster's turns from the next on,
        /// before the draw, costing (3); the opponent's turns add none.
        #[test]
        fn r3569007_r4154101_each_of_your_turns_starts_with_a_3_cost_one_and_the_opponents_add_none() {
            let mut s = game("expedition-turns", false);
            s.play(ID, json!({}));
            for turn in 1..=3 {
                s.end_turn();
                assert_eq!(s.state().active, PlayerId::P2);
                assert_eq!(added(&s).len(), turn - 1, "the opponent's turn adds none");
                s.end_turn();
                assert_eq!(s.state().active, P1);
                let made = added(&s);
                assert_eq!(made.len(), turn, "turn {turn}: {made:?}");
                expect_terminus(&s, &made, false, TERMINUS_COST);
                let events = s.last_events();
                let came = events.iter().position(
                    |event| matches!(event, GameEvent::AddedToHand { player: P1, def_id, .. } if *def_id != FILLER),
                );
                let drawn = events.iter().rposition(|event| matches!(event, GameEvent::Drawn { .. }));
                assert!(came.is_some() && came < drawn, "{events:?}");
                assert_eq!(s.state().players[P1].mana.max, 3, "one mana lost, once");
            }
        }

        /// R767000: after v0.6.5b through v0.6.6h it lost no mana and added a card costing (2) now and at
        /// each of its caster's turn starts (issue #4154).
        #[test]
        fn r767000_the_script_kept_for_v0_6_6h_adds_a_2_cost_card_now_and_each_turn_and_keeps_max_mana() {
            crate::under_kept(ID, script_v0_6_6h(), || {
                let mut s = game("expedition-kept-v0-6-6e", false);
                s.play(ID, json!({}));
                // Cast for (1) from 4: 3 left of a max that stays 4.
                assert_eq!(s.state().players[P1].mana.max, 4);
                assert_eq!(s.state().players[P1].mana.current, 3);
                let made = added(&s);
                assert_eq!(made.len(), 1, "{made:?}");
                expect_terminus(&s, &made, false, TERMINUS_COST_V0_6_6H);
                s.end_turn().end_turn();
                let made = added(&s);
                assert_eq!(made.len(), 2, "{made:?}");
                expect_terminus(&s, &made, false, TERMINUS_COST_V0_6_6H);
            });
        }

        /// R767000, R3569008: after v0.6.4c through v0.6.5b it lowered its caster's max mana for the rest of
        /// the game and added one card, costing (2) (issue #3996).
        #[test]
        fn r767000_r3569008_the_script_kept_for_v0_6_5b_lowers_max_mana_and_adds_one_2_cost_card() {
            kept(script_v0_6_5b(), || {
                let mut s = game("expedition-kept-v0-6-5", false);
                s.play(ID, json!({}));
                // Cast for (1) from 4: 3 left of a max that fell from 4 to 3.
                assert_eq!(s.state().players[P1].mana.max, 3);
                assert_eq!(s.state().players[P1].mana.current, 3);
                let made = added(&s);
                assert_eq!(made.len(), 1, "{made:?}");
                expect_terminus(&s, &made, false, TERMINUS_COST_V0_6_6H);
                s.end_turn().end_turn();
                assert_eq!(s.state().active, P1);
                // A turn on, the max stays lower and no card comes.
                assert_eq!(s.state().players[P1].mana.max, 3);
                assert_eq!(added(&s).len(), 1);
            });
        }

        /// R767000: through v0.6.4c the card added cost (3) (issue #3951).
        #[test]
        fn r767000_the_script_kept_for_v0_6_4c_adds_a_3_cost_terminus_card() {
            kept(script_v0_6_4c(), || {
                let mut s = game("expedition-kept", false);
                s.play(ID, json!({}));
                let made = added(&s);
                assert_eq!(made.len(), 1, "{made:?}");
                assert_eq!(effective_cost(s.state(), &made[0], Default::default()), TERMINUS_COST_V0_6_4C);
            });
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4154101_radiant_lowers_max_mana_by_1_and_adds_a_radiant_one_at_each_of_its_casters_turns() {
            for n in 0..6 {
                let mut s = game(&format!("expedition-radiant-{n}"), true);
                s.play(ID, json!({}));
                assert_eq!(s.state().players[P1].mana.max, 3, "seed {n}");
                assert!(added(&s).is_empty(), "seed {n}: no card now");
                s.end_turn().end_turn();
                let made = added(&s);
                assert_eq!(made.len(), 1, "seed {n}: {made:?}");
                expect_terminus(&s, &made, true, TERMINUS_COST);
            }
        }

        #[test]
        fn r458_its_badge_reads_its_radiant_words() {
            let mut s = game("expedition-radiant-badge", true);
            s.play(ID, json!({}));
            let label = fill_params(&crate::card_def(ID), FaceKind::Radiant, None);
            let mine: Vec<String> = s.view(P1).you.modifiers.iter().map(|m| m.label.clone()).collect();
            assert_eq!(mine, vec![label]);
        }
    }
}
