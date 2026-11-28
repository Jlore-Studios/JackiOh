//! SPEC §8.14 FvH #34 Half-True Craft a Card ((4) Spell, Mythic), a card of the held Felinors vs Humans set
//! (R5235000). Built by S09 (#5259).
//!   Base:    "Discover 2 cards. Fuse them and add the result to your hand. It costs (0)."
//!   Radiant: "Discover 3 cards. Fuse them and add the result to your hand. It costs (0)."
//!
//! Engine: Core #99 Craft a Card's chain (§10.6) with a pool of any type: each Discover a
//! `discover_from_catalog` with an empty query, the non-token cards of every shipped set (R380), never this
//! card (R387, the Discover's own rule), three options each, one Discover after another, the picks riding the
//! resume data. The last answer fuses them (`fuse_cards`, R102) into one transient definition, to the
//! caster's hand at (0) (R77, R352). A Spell fused with a Unit takes the first pick's type (`subsystems/
//! fuse.rs`' fallback). Its 2, 3 and (0) are the engine's Discover and Fuse table, as Core #99's, fixed
//! (R1748003, `tests/cross/params.rs`). Tunes: none.

use jackioh_engine::effects::{chosen_options, discover_from_catalog, fuse_cards};
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-034";

/// One resume step per Discover, named for the Discover whose answer it receives (§10.6).
const FIRST: &str = "first";
const SECOND: &str = "second";
const THIRD: &str = "third";

/// The `data` key the picks travel in.
const PICKS: &str = "picks";

/// How many Discovers each face makes before the fusion.
const DISCOVERS: usize = 2;
const RADIANT_DISCOVERS: usize = 3;

/// §6.3 Discover: 1 of 3 cards of any type. An empty query draws no token (§5.1).
fn discover_card(step: &'static str, picks: &[String]) -> Effect {
    discover_from_catalog(json_as(json!({
        "step": step,
        "query": {},
        "prompt": "Discover a card",
        "data": { PICKS: picks },
    })))
}

/// The ids picked so far, read back out of the captured data.
fn picks_of(ctx: &EffectContext<'_>) -> Vec<String> {
    let Some(Value::Array(stored)) = ctx.data.get(PICKS) else {
        return vec![];
    };
    stored
        .iter()
        .filter_map(|entry| entry.as_str().map(str::to_string))
        .collect()
}

/// Those ids plus the one this step was answered with.
fn with_answer(ctx: &EffectContext<'_>) -> Vec<String> {
    let mut picks = picks_of(ctx);
    if let Some(picked) = chosen_options(ctx).into_iter().next() {
        picks.push(picked);
    }
    picks
}

/// R77: the picks fused into a fresh hand card at (0); fewer than two is no fusion, and the Spell still
/// counts as played (§8 Conventions).
fn craft(picks: &[String]) -> Vec<Effect> {
    if picks.len() < subsystems::FUSE_MIN_INGREDIENTS {
        return vec![];
    }
    vec![fuse_cards(json_as(json!({ "defIds": picks, "toHand": "self" })))]
}

fn half_true(discovers: usize) -> Script {
    let open_second: Hook = hook(|ctx| vec![discover_card(SECOND, &with_answer(ctx))]);
    let open_third: Hook = hook(|ctx| vec![discover_card(THIRD, &with_answer(ctx))]);
    let fuse_them: Hook = hook(|ctx| craft(&with_answer(ctx)));

    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    resume.insert(FIRST, open_second);
    if discovers == RADIANT_DISCOVERS {
        resume.insert(SECOND, open_third);
        resume.insert(THIRD, fuse_them);
    } else {
        resume.insert(SECOND, fuse_them);
    }

    Script {
        cry: Some(hook(|_ctx| vec![discover_card(FIRST, &[])])),
        resume,
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: half_true(DISCOVERS),
        radiant: half_true(RADIANT_DISCOVERS),
    }
}

// FvH #34 Half-True Craft a Card — SPEC §8.14 row 34 (R77, R102, R352, R380, R387, R1748003).
#[cfg(test)]
mod tests {
    use super::{DISCOVERS, ID, RADIANT_DISCOVERS};
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SPARE: &str = "core-005"; // Stockpile: a card kept in hand, so no turn ends by itself

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "half-true-craft",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, SPARE], "library": crate::filler(3), "mana": 10 },
            "p2": { "hand": [SPARE], "library": crate::filler(3) },
        }))
    }

    /// Plays the card and answers each Discover with its first option, checking every offer on the way:
    /// three options, each a non-token catalog card and never this one. Returns the picks.
    fn crafted(radiant: bool) -> (Scenario, Vec<String>) {
        let discovers = if radiant { RADIANT_DISCOVERS } else { DISCOVERS };
        let mut s = board(radiant);
        s.play(ID, json!({}));
        let mut picks = Vec::new();
        for step in 0..discovers {
            let pending = must(s.state().pending.clone(), "a Discover");
            assert_eq!(pending.kind, PromptKind::Discover, "Discover {}", step + 1);
            assert_eq!(pending.player_id, P1);
            let offered = s.prompt_modes();
            assert_eq!(offered.len(), DISCOVER_OPTIONS as usize, "Discover {}", step + 1);
            for id in nonempty(offered.clone(), "the options") {
                assert_ne!(id, ID, "R387: never this card");
                assert!(!crate::card_def(&id).token, "§5.1: no token is offered");
            }
            let chosen = offered[0].clone();
            s.answer(pick::mode(&chosen));
            picks.push(chosen);
        }
        (s, picks)
    }

    /// The one transient definition the resolution wrote (R77).
    fn fused_def_of(s: &Scenario) -> CardDef {
        let defs: Vec<&CardDef> = s.state().transient_defs.values().collect();
        assert_eq!(defs.len(), 1, "one fusion, one transient definition");
        defs[0].clone()
    }

    fn crafted_card(s: &Scenario, fused: &CardDef) -> CardInstance {
        must(
            s.hand(P1).into_iter().find(|card| card.def_id == fused.id),
            "the crafted card in hand",
        )
    }

    #[test]
    fn declares_no_numbers() {
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn r77_two_discovers_fuse_into_a_hand_card_that_costs_0() {
            let (mut s, picks) = crafted(false);
            assert_eq!(picks.len(), 2);
            assert!(s.state().pending.is_none(), "two Discovers, then the fusion");
            let fused = fused_def_of(&s);
            let names: Vec<String> = picks.iter().map(|id| crate::card_def(id).name.clone()).collect();
            assert_eq!(fused.name, names.join(" + "), "R102: both picks are in it");
            let card = crafted_card(&s, &fused);
            assert_eq!(effective_cost(s.state(), &card, Default::default()), 0);
            assert!(!card.radiant, "R77: a fresh card, not Radiant");
            s.expect_in_zone(ID, "graveyard");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r77_three_discovers_fuse_into_a_hand_card_that_costs_0() {
            let (s, picks) = crafted(true);
            assert_eq!(picks.len(), 3);
            assert!(s.state().pending.is_none(), "three Discovers, then the fusion");
            let fused = fused_def_of(&s);
            let names: Vec<String> = picks.iter().map(|id| crate::card_def(id).name.clone()).collect();
            assert_eq!(fused.name, names.join(" + "), "R102: all three picks are in it");
            let card = crafted_card(&s, &fused);
            assert_eq!(effective_cost(s.state(), &card, Default::default()), 0);
        }
    }
}
