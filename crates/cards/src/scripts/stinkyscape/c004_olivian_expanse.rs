//! S #4 Olivian Expanse (SPEC §8.10 row S 4; §2.4, §6.3 Discover, §10.8, R70, R382, R850, R1420,
//! R1837000, R1857002, R1870000, R3869000): (4) Spell, Mythic.
//!   Base:    "Discover a Terminus card. If your deck is empty, it costs (0)."
//!   Radiant: "… If this was also the last card in your hand, you win the game when you play it."
//!
//! Engine: `discover_from_catalog` over the Terminus pool (`query::terminus_cards`, R3869000), every
//! card and token tagged Terminus of the sets that ship, three different options shown to its caster
//! alone: Core #100 Ceaseless Void (tagged Terminus since R1837000), the set's six Terminus tokens
//! (R1857002) and the Stinkyscape+ set's Terminus cards and tokens. The pick reaches the hand on its
//! base face. Through v0.6.0g the pool named the first seven by id, as it had to while the set was held
//! (R1420; `script_v0_6_0g`, kept, R767000).

use jackioh_engine::config::DISCOVER_OPTIONS;
use jackioh_engine::effects::{WIN_ON_PLAY_KEY, add_to_hand, discover_from_catalog};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-004";

/// R767000: the Terminus cards through v0.6.0g, Core #100 Ceaseless Void and the set's six Terminus
/// tokens, in catalog order, which `script_v0_6_0g` and Stinkyscape #154 Dice of Doom's kept script name.
pub const TERMINUS_CARDS_V0_6_0G: [&str; 7] = [
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

/// The Discover; `kept`, the script through v0.6.0g.
fn discover(ctx: &mut EffectContext<'_>, kept: bool) -> Vec<Effect> {
    let free = zone_count(ctx.state, ctx.controller, OffFieldZone::Library) == 0;
    let wins = ctx.radiant && free && zone_count(ctx.state, ctx.controller, OffFieldZone::Hand) == 0;
    // R3869000: the Terminus pool; R767000: through v0.6.0g, the seven named by id.
    let pool = if kept { json!({ "defId": TERMINUS_CARDS_V0_6_0G }) } else { json!(crate::query::terminus_cards()) };
    vec![discover_from_catalog(json_as(json!({
        "step": PICKED,
        "count": DISCOVER_OPTIONS,
        "query": pool,
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

/// Both faces; `kept`, the script through v0.6.0g.
fn faces(kept: bool) -> CardScripts {
    let base = Script {
        cry: Some(hook(move |ctx| discover(ctx, kept))),
        resume: IndexMap::from([(PICKED, hook(picked))]),
        ..Script::default()
    };
    // The face is read off the context, so both faces run the one script.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

pub fn script() -> CardScripts { faces(false) }

/// R767000: through v0.6.0g the Discover offered the seven Terminus cards it named by id; `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_6_0g() -> CardScripts { faces(true) }

// S #4 Olivian Expanse — SPEC §8.10 row S 4, BUILD M12 row S 4 (R1870000).
#[cfg(test)]
mod tests {
    use super::{ID, TERMINUS_CARDS_V0_6_0G, script_v0_6_0g};
    use jackioh_engine::config::{DISCOVER_OPTIONS, HAND_CAP};
    use jackioh_engine::effects::WIN_ON_PLAY_KEY;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-005"; // Stockpile: a Spell
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

    /// The open prompt is p1's Discover.
    fn assert_discover(s: &Scenario) {
        let pending = s.state().pending.as_ref().expect("a Discover is open");
        assert_eq!((pending.kind, pending.player_id), (PromptKind::Discover, P1));
    }

    /// R3869000: the Terminus pool's ids, in its order.
    fn terminus() -> Vec<String> {
        crate::register_all();
        crate::query::pool_ids(&crate::query::terminus_cards())
    }

    /// Answers the open Discover with a playable pick when one is offered, else with the first option.
    fn pick(s: &mut Scenario) -> Option<CardInstance> {
        assert_discover(s);
        let options = s.prompt_modes();
        let choice = PLAYABLE
            .iter()
            .map(|id| id.to_string())
            .find(|id| options.contains(id))
            .unwrap_or_else(|| options[0].clone());
        s.answer(json!(choice));
        s.hand(P1).into_iter().find(|card| card.def_id == choice)
    }

    /// The first seed `{seed}-{n}` whose Discover offers a playable pick (the pool holds Terminus cards
    /// these tests do not play), the scenario at that Discover.
    fn expanse_offering_a_playable(seed: &str, radiant: bool, hand: Value, library: Value) -> Scenario {
        (0..64)
            .map(|n| expanse(&format!("{seed}-{n}"), radiant, hand.clone(), library.clone()))
            .find(|s| s.prompt_modes().iter().any(|offered| PLAYABLE.contains(&offered.as_str())))
            .expect("a seed offers a playable pick")
    }

    mod r1870000_olivian_expanse {
        use super::*;

        #[test]
        fn r1870000_discovers_three_different_terminus_cards() {
            for i in 0..10 {
                let s = expanse(&format!("olivian-{i}"), false, json!([]), json!([FILLER]));
                assert_discover(&s);
                let options = s.prompt_modes();
                assert_eq!(options.len(), DISCOVER_OPTIONS as usize);
                let mut distinct = options.clone();
                distinct.sort();
                distinct.dedup();
                assert_eq!(distinct.len(), options.len(), "three different: {options:?}");
                for option in &options {
                    assert!(terminus().contains(option), "{option} is a Terminus card");
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
            assert_discover(&s);
            let options = s.prompt_modes();
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
            let mut s = expanse_offering_a_playable("olivian-wins", true, json!([]), json!([]));
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
            let mut s = expanse_offering_a_playable("olivian-held", true, json!([FILLER]), json!([]));
            let card = pick(&mut s).expect("the pick is in hand");
            assert!(!card.memory.contains_key(WIN_ON_PLAY_KEY));
            assert_eq!(card.cost_override, Some(0), "it is still free");
            s.play(card.id.as_str(), json!({}));
            assert!(s.state().result.is_none());
        }

        #[test]
        fn r1870000_a_hero_at_zero_still_loses_first() {
            let mut s = expanse_offering_a_playable("olivian-zero", true, json!([]), json!([]));
            let card = pick(&mut s).expect("the pick is in hand");
            s.state_mut().players.p1.hero.health = 0;
            s.play(card.id.as_str(), json!({}));
            let result = s.state().result.expect("the game is over");
            assert_eq!(result.winner, Winner::P2, "a hero at 0 loses before a win is held");
        }
    }

    mod r3869000_the_terminus_pool {
        use super::*;

        #[test]
        fn r3869000_the_pool_is_every_terminus_card_and_token_of_the_sets_that_ship() {
            let pool = terminus();
            for id in TERMINUS_CARDS_V0_6_0G {
                assert!(pool.contains(&id.to_string()), "{id} is still in it");
            }
            for id in ["stinkyscapeplus-077", "stinkyscapeplus-080", "stinkyscapeplus-t-supernova"] {
                assert!(pool.contains(&id.to_string()), "{id} joined it with its set");
            }
            for id in &pool {
                assert!(crate::card_def(id).tags.contains(&Tag::Terminus), "{id} is tagged Terminus");
            }
            assert!(!pool.contains(&ID.to_string()), "Olivian Expanse is no Terminus card");
        }

        #[test]
        fn r3869000_the_discover_can_offer_a_stinkyscape_plus_terminus_card() {
            let offered: Vec<String> = (0..24)
                .flat_map(|n| expanse(&format!("olivian-plus-{n}"), false, json!([]), json!([FILLER])).prompt_modes())
                .collect();
            assert!(offered.iter().any(|id| id.starts_with("stinkyscapeplus-")), "some seed offers one: {offered:?}");
            let pool = terminus();
            assert!(offered.iter().all(|id| pool.contains(id)));
        }

        #[test]
        fn r767000_the_script_kept_for_v0_6_0g_offers_only_the_seven_it_named() {
            crate::under_kept(ID, script_v0_6_0g(), || {
                for n in 0..12 {
                    let s = expanse(&format!("olivian-kept-{n}"), false, json!([]), json!([FILLER]));
                    for option in s.prompt_modes() {
                        assert!(TERMINUS_CARDS_V0_6_0G.contains(&option.as_str()), "{option}");
                    }
                }
            });
        }
    }
}
