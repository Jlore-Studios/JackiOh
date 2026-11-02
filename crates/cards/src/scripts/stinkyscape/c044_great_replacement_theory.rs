//! S #44 Great Replacement Theory (SPEC §8.10 row 44): (1) Spell, Quickdraw, Mythic, a card of the
//! Stinkyscape set. Its name is the designer's satire, kept.
//!   Base:    "Add a Drone Core I to your hand. For the rest of the game, the first time you play each tier
//!            of Drone Core, add the next tier to your hand."
//!   Radiant: "Add a Radiant Drone Core I to your hand. For the rest of the game, the first time you play
//!            each tier of Drone Core, add a Radiant one of the next tier to your hand."
//! Quickdraw deals in opening hand (R640). Adds Drone Core I (S #27).
//! Sets watch_first_plays on caster (R1863000, §10.1, R70, S #28–#36, §2.4, R127, R169, R767000): next tier added on first play of each tier.

use jackioh_engine::effects::{WatchFirstPlaysArgs, watch_first_plays};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-044";

/// Drone Core I to X (S #27–#36), in tier order.
const TIERS: [&str; 10] = [
    "stinkyscape-027",
    "stinkyscape-028",
    "stinkyscape-029",
    "stinkyscape-030",
    "stinkyscape-031",
    "stinkyscape-032",
    "stinkyscape-033",
    "stinkyscape-034",
    "stinkyscape-035",
    "stinkyscape-036",
];

/// The step the watcher re-enters for each tier's first resolved play.
const NEXT_TIER: &str = "next-tier";

/// R1863000: the tier after the one the watcher answered (`defId`), Radiant on `radiant`; none after X.
fn next_tier(ctx: &EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    let Some(played) = ctx.data.get("defId").and_then(Value::as_str) else {
        return vec![];
    };
    let Some(next) = TIERS
        .iter()
        .position(|tier| *tier == played)
        .and_then(|at| TIERS.get(at + 1))
    else {
        return vec![];
    };
    vec![add_to_hand(json_as(json!({ "defId": next, "radiant": radiant })))]
}

fn face(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |ctx| {
            let face = if radiant { FaceKind::Radiant } else { FaceKind::Base };
            let label = fill_params(def_of(Some(&*ctx.state), ID), face, None);
            vec![
                add_to_hand(json_as(json!({ "defId": TIERS[0], "radiant": radiant }))),
                watch_first_plays(WatchFirstPlaysArgs {
                    // X adds no tier, so the watch names I to IX.
                    def_ids: TIERS[..TIERS.len() - 1].iter().map(|tier| tier.to_string()).collect(),
                    step: NEXT_TIER.to_string(),
                    label,
                    hook: None,
                    data: None,
                }),
            ]
        })),
        resume: IndexMap::from([(NEXT_TIER, hook(move |ctx| next_tier(ctx, radiant)))]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S #44 Great Replacement Theory — SPEC §8.10 row 44, BUILD M12 row S 44 (R1863000, R640).
#[cfg(test)]
mod tests {
    use super::{ID, TIERS, script};
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const THEORY: &str = "stinkyscape-044";
    const CORE_II: &str = "stinkyscape-028";
    const CORE_III: &str = "stinkyscape-029";
    const COUNTERSPELL: &str = "classic-017"; // (2) Trap: counter the opponent's Spell.
    const FILLER: &str = "core-005";

    fn setup(hand: Value, p2: Value) -> Scenario {
        let mut cards = hand.as_array().cloned().unwrap_or_default();
        cards.extend([json!(FILLER), json!(FILLER), json!(FILLER)]);
        let mut p2 = p2;
        if p2.get("hand").is_none() {
            p2["hand"] = json!([FILLER]);
        }
        p2["library"] = filler(6);
        scenario(json!({
            "seed": "great-replacement",
            "p1": { "hand": cards, "library": filler(6), "mana": 40 },
            "p2": p2,
        }))
    }

    fn held(s: &Scenario, player: PlayerId, def_id: &str) -> Vec<bool> {
        s.hand(player)
            .into_iter()
            .filter(|card| card.def_id == def_id)
            .map(|card| card.radiant)
            .collect()
    }

    fn cores(s: &Scenario, player: PlayerId) -> usize {
        s.hand(player)
            .iter()
            .filter(|card| TIERS.contains(&card.def_id.as_str()))
            .count()
    }

    fn watched(s: &Scenario) -> Vec<Vec<String>> {
        s.state().players.p1.mods
            .iter()
            .filter_map(|modifier| match &modifier.kind {
                ModifierKind::PlayWatch { def_ids, .. } => Some(def_ids.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r640_quickdraw_deals_it_in_the_opening_hand() {
        crate::register_all();
        let scripts = script();
        assert_eq!(scripts.base.flags().quickdraw, Some(true));
        assert_eq!(scripts.radiant.flags().quickdraw, Some(true));
        // Plain Core cards, then Great Replacement Theory: one copy of each (§2.6 L3).
        let others = [
            "core-002", "core-005", "core-006", "core-008", "core-011", "core-012", "core-013", "core-015",
            "core-016", "core-019", "core-020", "core-025", "core-026", "core-032", "core-036", "core-043",
            "core-044", "core-053", "core-055", "core-001", "core-003", "core-004", "core-009", "core-010",
        ];
        let mut deck: Vec<String> = others.iter().map(|id| id.to_string()).collect();
        deck.push(ID.to_string());
        let created = create_game(&CreateGameOptions {
            seed: "great-replacement-quickdraw".to_string(),
            decks: (deck.clone(), deck),
            ..CreateGameOptions::default()
        });
        let state = begin_game(&created).state;
        assert!(state.players.p1.hand.iter().any(|card| card.def_id == ID));
        assert!(state.players.p1.library.iter().all(|card| card.def_id != ID));
    }

    #[test]
    fn r1863000_each_first_play_adds_the_next_tier_up_to_x() {
        let mut s = setup(json!([THEORY]), json!({}));
        s.play(THEORY, json!({}));
        for (at, tier) in TIERS.iter().enumerate() {
            assert_eq!(held(&s, P1, tier), vec![false], "tier {} is in hand", at + 1);
            assert_eq!(cores(&s, P1), 1);
            s.play(*tier, json!({}));
        }
        assert_eq!(cores(&s, P1), 0, "Drone Core X adds nothing");
        assert_eq!(watched(&s), vec![Vec::<String>::new()], "every tier answered once");
    }

    #[test]
    fn r1863000_a_second_play_of_a_tier_adds_nothing() {
        let mut s = setup(json!([THEORY, CORE_II, CORE_II]), json!({}));
        s.play(THEORY, json!({}));
        s.play(CORE_II, json!({}));
        assert_eq!(held(&s, P1, CORE_III), vec![false]);
        s.play(CORE_II, json!({}));
        assert_eq!(held(&s, P1, CORE_III), vec![false], "still the one");
    }

    #[test]
    fn r1863000_a_drone_core_played_before_it_counts_for_nothing() {
        let mut s = setup(json!([CORE_II, THEORY, CORE_II]), json!({}));
        s.play(CORE_II, json!({}));
        assert!(held(&s, P1, CORE_III).is_empty());
        s.play(THEORY, json!({}));
        s.play(CORE_II, json!({}));
        assert_eq!(held(&s, P1, CORE_III), vec![false], "the first play since it counts");
    }

    #[test]
    fn r1863000_a_countered_play_adds_nothing_and_its_tier_stays_watched() {
        let mut s = setup(
            json!([THEORY, CORE_II, CORE_II]),
            json!({ "hand": [COUNTERSPELL, FILLER] }),
        );
        s.play(THEORY, json!({}));
        s.end_turn();
        s.play(COUNTERSPELL, json!({}));
        s.end_turn();
        s.play(CORE_II, json!({}));
        s.expect_events(json!(["trapFired", "countered"]));
        assert!(held(&s, P1, CORE_III).is_empty(), "the countered play never resolved");
        assert!(watched(&s)[0].contains(&CORE_II.to_string()));
        s.play(CORE_II, json!({}));
        assert_eq!(held(&s, P1, CORE_III), vec![false]);
    }

    #[test]
    fn r1863000_two_casts_add_two() {
        let mut s = setup(json!([THEORY, THEORY, CORE_II, CORE_II]), json!({}));
        s.play(THEORY, json!({}));
        s.play(THEORY, json!({}));
        assert_eq!(held(&s, P1, TIERS[0]), vec![false, false]);
        s.play(CORE_II, json!({}));
        assert_eq!(held(&s, P1, CORE_III), vec![false, false], "each cast keeps its own record");
        s.play(CORE_II, json!({}));
        assert_eq!(held(&s, P1, CORE_III).len(), 2);
    }

    #[test]
    fn r1863000_a_full_hand_burns_the_next_tier_and_the_tier_is_spent() {
        let mut s = scenario(json!({
            "p1": {
                "hand": [THEORY, THEORY, CORE_II, CORE_II, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                "library": filler(6),
                "mana": 40,
            },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }));
        s.play(THEORY, json!({}));
        s.play(THEORY, json!({}));
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
        s.play(CORE_II, json!({}));
        assert_eq!(held(&s, P1, CORE_III), vec![false]);
        assert!(s.last_events().iter().any(
            |event| matches!(event, GameEvent::Burned { def_id, .. } if def_id == CORE_III)
        ));
        assert!(watched(&s).iter().all(|watch| !watch.contains(&CORE_II.to_string())));
        s.play(CORE_II, json!({}));
        assert_eq!(held(&s, P1, CORE_III), vec![false], "the tier was spent");
    }

    #[test]
    fn r1863000_the_opponents_play_adds_nothing() {
        let mut s = setup(json!([THEORY]), json!({ "hand": [CORE_II, FILLER] }));
        s.play(THEORY, json!({}));
        s.end_turn();
        s.play(CORE_II, json!({}));
        assert!(held(&s, P1, CORE_III).is_empty());
        assert!(held(&s, P2, CORE_III).is_empty());
        assert!(watched(&s)[0].contains(&CORE_II.to_string()));
    }

    #[test]
    fn r1863000_r169_the_badge_is_in_both_views() {
        let mut s = setup(json!([THEORY]), json!({}));
        s.play(THEORY, json!({}));
        let text = crate::card_def(THEORY).base.text;
        let labels = |modifiers: &[ModifierView]| -> Vec<String> {
            modifiers.iter().map(|modifier| modifier.label.clone()).collect()
        };
        assert_eq!(labels(&s.view(P1).you.modifiers), vec![text.clone()]);
        assert_eq!(labels(&s.view(P2).opponent.modifiers), vec![text]);
    }

    #[test]
    fn r1863000_radiant_every_card_it_adds_is_radiant() {
        let mut s = setup(json!([{ "def": THEORY, "radiant": true }]), json!({}));
        s.play(THEORY, json!({}));
        assert_eq!(held(&s, P1, TIERS[0]), vec![true]);
        s.play(TIERS[0], json!({}));
        assert_eq!(held(&s, P1, CORE_II), vec![true]);
        s.play(CORE_II, json!({}));
        assert_eq!(held(&s, P1, CORE_III), vec![true]);
        let text = crate::card_def(THEORY).radiant.text;
        assert_eq!(s.view(P1).you.modifiers[0].label, text);
    }
}
