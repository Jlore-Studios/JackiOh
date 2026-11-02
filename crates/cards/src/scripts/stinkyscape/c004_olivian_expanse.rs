//! S #4 Olivian Expanse (SPEC §8.10 row S 4; §2.4, §6.3 Discover, §10.8, R70, R382, R850, R1420,
//! R1837000, R1857002, R1870000): (4) Spell, Mythic.
//!   Base:    "Discover a Terminus card. If your deck is empty, it costs (0)."
//!   Radiant: "… If this was also the last card in your hand, you win the game when you play it."
//!
//! Engine: `discover_from_catalog` over the Terminus cards (`TERMINUS_CARDS`), three different options
//! shown to its caster alone: the set's six Terminus tokens (R1857002) and Core #100 Ceaseless Void
//! (tagged Terminus since R1837000). Terminus is no `POOL_TOKEN_TAGS` tag (R382), so the pool names its
//! cards by id, which reaches them whether or not the set ships (R1420). The pick reaches the hand on its
//! base face.

use jackioh_engine::config::DISCOVER_OPTIONS;
use jackioh_engine::effects::{WIN_ON_PLAY_KEY, add_to_hand, discover_from_catalog};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-004";

/// The Terminus cards: Core #100 Ceaseless Void and the set's six Terminus tokens, in catalog order.
/// Stinkyscape #154 Dice of Doom adds one of each.
pub const TERMINUS_CARDS: [&str; 7] = [
    "core-100",
    "stinkyscape-004-1",
    "stinkyscape-004-2",
    "stinkyscape-004-3",
    "stinkyscape-004-4",
    "stinkyscape-004-5",
    "stinkyscape-004-6",
];

/// The Discover's answer comes back here (§10.6).
const PICKED: &str = "picked";
/// The step data: the library was empty as it resolved, and (Radiant) the hand too.
const FREE: &str = "free";
const WINS: &str = "wins";

fn discover(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let free = zone_count(ctx.state, ctx.controller, OffFieldZone::Library) == 0;
    let wins = ctx.radiant && free && zone_count(ctx.state, ctx.controller, OffFieldZone::Hand) == 0;
    vec![discover_from_catalog(json_as(json!({
        "step": PICKED,
        "count": DISCOVER_OPTIONS,
        "query": { "defId": TERMINUS_CARDS },
        "prompt": "Discover a Terminus card",
        "data": { FREE: free, WINS: wins },
    })))]
}

/// The pick to the hand, on its base face, at (0) and carrying the win as the data says.
fn picked(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(picked) = chosen_options(ctx).into_iter().next() else {
        return vec![];
    };
    let flag = |key: &str| ctx.data.get(key).and_then(Value::as_bool) == Some(true);
    let mut args = json!({ "defId": picked });
    if flag(FREE) {
        args["costOverride"] = json!(0);
    }
    if flag(WINS) {
        args["memory"] = json!({ WIN_ON_PLAY_KEY: true });
    }
    vec![add_to_hand(json_as(args))]
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(discover)),
        resume: IndexMap::from([(PICKED, hook(picked))]),
        ..Script::default()
    };
    // The face is read off the context, so both faces run the one script.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S #4 Olivian Expanse — SPEC §8.10 row S 4, BUILD M12 row S 4 (R1870000).
#[cfg(test)]
mod tests {
    use super::{ID, TERMINUS_CARDS};
    use jackioh_engine::config::{DISCOVER_OPTIONS, HAND_CAP};
    use jackioh_engine::effects::WIN_ON_PLAY_KEY;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-005"; // (1) Spell Stockpile.
    const PLAYABLE: [&str; 5] = [
        "stinkyscape-004-3",
        "stinkyscape-004-1",
        "stinkyscape-004-4",
        "stinkyscape-004-5",
        "core-100",
    ];

    fn expanse(seed: &str, radiant: bool, hand: Value, library: Value) -> Scenario {
        let mut cards = vec![json!({ "def": ID, "radiant": radiant })];
        cards.extend(hand.as_array().cloned().unwrap_or_default());
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": cards, "library": library, "mana": 10 },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }));
        s.state_mut().players.p1.auto_end_turn = Some(false);
        s.play(ID, json!({}));
        s
    }

    fn offered(s: &Scenario) -> Vec<String> {
        let pending = s.state().pending.clone().expect("a Discover is open");
        assert_eq!((pending.kind, pending.player_id), (PromptKind::Discover, P1));
        pending
            .options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Mode { option } => Some(option.clone()),
                _ => None,
            })
            .collect()
    }

    fn pick(s: &mut Scenario) -> Option<CardInstance> {
        let options = offered(s);
        let choice = PLAYABLE
            .iter()
            .find(|id| options.iter().any(|offered| offered == *id))
            .expect("one playable pick is offered");
        s.answer(json!(choice));
        s.hand(P1).into_iter().find(|card| card.def_id == *choice)
    }

    mod r1870000_olivian_expanse {
        use super::*;

        #[test]
        fn r1870000_discovers_three_different_terminus_cards() {
            for i in 0..10 {
                let s = expanse(&format!("olivian-{i}"), false, json!([]), json!([FILLER]));
                let options = offered(&s);
                assert_eq!(options.len(), DISCOVER_OPTIONS as usize);
                let mut distinct = options.clone();
                distinct.sort();
                distinct.dedup();
                assert_eq!(distinct.len(), options.len(), "three different: {options:?}");
                for option in &options {
                    assert!(TERMINUS_CARDS.contains(&option.as_str()), "{option} is a Terminus card");
                }
                // The other seat sees only that p1 is choosing (§10.8).
                assert!(matches!(
                    s.view(P2).pending,
                    Some(PendingView::Elsewhere(PendingElsewhereView { pending_for: P1, .. }))
                ));
            }
        }

        #[test]
        fn r1870000_empty_deck_pick_costs_zero() {
            let mut s = expanse("olivian-free", false, json!([FILLER]), json!([]));
            let card = pick(&mut s).expect("the pick is in hand");
            assert_eq!(card.cost_override, Some(0));
            assert!(!card.memory.contains_key(WIN_ON_PLAY_KEY), "the base face carries no win");
        }

        #[test]
        fn r1870000_deck_left_pick_costs_printed() {
            let mut s = expanse("olivian-printed", false, json!([]), json!([FILLER]));
            let card = pick(&mut s).expect("the pick is in hand");
            assert_eq!(card.cost_override, None);
        }

        #[test]
        fn r1870000_full_hand_burns_the_pick() {
            let held: Vec<&str> = vec![FILLER; HAND_CAP as usize];
            let mut s = expanse("olivian-full", false, json!(held), json!([]));
            let graveyard = s.pile(P1, "graveyard").len();
            let options = offered(&s);
            s.answer(json!(options[0]));
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            assert!(s.hand(P1).iter().all(|card| card.def_id == FILLER), "the pick is not in hand");
            let burned: Vec<String> = s.pile(P1, "graveyard").into_iter().map(|card| card.def_id).collect();
            assert!(burned.len() > graveyard && burned.contains(&options[0]), "it burned: {burned:?}");
        }

        #[test]
        fn r1870000_pick_is_base_face() {
            let mut s = expanse("olivian-face", true, json!([FILLER]), json!([]));
            let card = pick(&mut s).expect("the pick is in hand");
            assert!(!card.radiant, "the Radiant face adds its pick on the base face");
        }

        #[test]
        fn r1870000_radiant_last_card_wins_on_play() {
            let mut s = expanse("olivian-wins", true, json!([]), json!([]));
            let card = pick(&mut s).expect("the pick is in hand");
            assert_eq!(card.memory.get(WIN_ON_PLAY_KEY), Some(&Value::Bool(true)));
            assert_eq!(card.cost_override, Some(0));
            assert!(s.state().result.is_none(), "holding it wins nothing");
            s.play(card.id.as_str(), json!({}));
            let result = s.state().result.expect("its play wins");
            assert_eq!(result.winner, Winner::P1);
            assert_eq!(result.reason, GameOverReason::WonByEffect);
        }

        #[test]
        fn r1870000_radiant_with_a_card_left_in_hand_wins_nothing() {
            let mut s = expanse("olivian-held", true, json!([FILLER]), json!([]));
            let card = pick(&mut s).expect("the pick is in hand");
            assert!(!card.memory.contains_key(WIN_ON_PLAY_KEY));
            assert_eq!(card.cost_override, Some(0), "it is still free");
            s.play(card.id.as_str(), json!({}));
            assert!(s.state().result.is_none());
        }

        #[test]
        fn r1870000_a_hero_at_zero_still_loses_first() {
            let mut s = expanse("olivian-zero", true, json!([]), json!([]));
            let card = pick(&mut s).expect("the pick is in hand");
            s.state_mut().players.p1.hero.health = 0;
            s.play(card.id.as_str(), json!({}));
            let result = s.state().result.expect("the game is over");
            assert_eq!(result.winner, Winner::P2, "a hero at 0 loses before a win is held");
        }
    }
}
