//! M #62 Small Time Recruits (SPEC §8.8 row 62): (0) Spell, Rare.
//!   Base:    "Draw {cards|(1) Cost Unit|(1) Cost Units} from your deck." — cards 3
//!   Radiant: the same, then "Make them Radiant."
//! Engine: three different random Units costing (1) in the deck (R65/R66, X excluded, MD-D17, R1061),
//!   each a named §2.4 draw (R113's set read once); radiant makes each still in hand Radiant.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-062";

/// The one cost the card names.
const RECRUIT_COST: i32 = 1;

fn recruits(ctx: &mut EffectContext<'_>) -> Vec<String> {
    let count = param(&*ctx, "cards").max(0) as usize;
    let matching: Vec<CardInstance> = zone_cards(ctx.state, ctx.controller, OffFieldZone::Library)
        .into_iter()
        .filter(|card| {
            card_type_of(ctx.state, card) == CardType::Unit
                && !is_x_cost(ctx.state, card)
                && effective_cost(ctx.state, card, Default::default()) == RECRUIT_COST
        })
        .collect();
    if matching.len() <= count {
        return matching.into_iter().map(|card| card.id).collect();
    }
    ctx.rng.shuffle(&matching).into_iter().take(count).map(|card| card.id).collect()
}

fn recruits_face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(recruits),
                each: Arc::new(move |id| {
                    draw_from_library(json_as(json!({ "instanceId": id, "radiant": radiant })))
                }),
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: recruits_face(false),
        radiant: recruits_face(true),
    }
}

// M #62 Small Time Recruits — SPEC §8.8 row 62, BUILD M10 row M 62.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SMALL: &str = "meditative-062";
    const FILLER: &str = "core-005";

    fn drawn_ids(s: &Scenario) -> Vec<String> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Drawn { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r1061_draws_three_different_cost_1_units() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [SMALL], "library": ["core-008", "core-011", "core-003", "core-086", "core-005", "core-020", "classicplus-069"] },
            "p2": { "hand": [FILLER] },
        }));
        s.play(SMALL, json!({}));
        let drawn = drawn_ids(&s);
        assert_eq!(drawn.len(), 3);
        let defs: Vec<String> = drawn.iter().map(|id| s.card(id).def_id.clone()).collect();
        assert_eq!(defs.iter().collect::<IndexSet<_>>().len(), 3);
        for id in &drawn {
            let card = s.card(id);
            assert_eq!(card_type_of(s.state(), card), CardType::Unit);
            assert_eq!(effective_cost(s.state(), card, Default::default()), 1);
        }
    }

    #[test]
    fn r1061_fewer_matches_draw_fewer_no_fatigue() {
        crate::register_all();
        let health = 30;
        let mut s = scenario(json!({
            "p1": { "hand": [SMALL], "library": ["core-008", "core-005"] },
            "p2": { "hand": [FILLER] },
        }));
        s.play(SMALL, json!({}));
        assert_eq!(drawn_ids(&s).len(), 1);
        assert_eq!(s.state().players.p1.hero.health, health);
    }

    #[test]
    fn r66_r1061_the_cost_is_read_in_the_deck() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [SMALL], "library": ["core-020", "core-008", "core-005", "core-011"] },
            "p2": { "hand": [FILLER] },
        }));
        {
            let library = s.pile(PlayerId::P1, "library");
            let discounted = library.iter().find(|card| card.def_id == "core-020").expect("core-020").id.clone();
            let inflated = library.iter().find(|card| card.def_id == "core-008").expect("core-008").id.clone();
            find_instance_mut(s.state_mut(), &discounted).expect("in the library").cost_mod = -1;
            find_instance_mut(s.state_mut(), &inflated).expect("in the library").cost_mod = 1;
        }
        s.play(SMALL, json!({}));
        let defs: Vec<String> = drawn_ids(&s).iter().map(|id| s.card(id).def_id.clone()).collect();
        assert!(defs.contains(&"core-020".to_string()));
        assert!(!defs.contains(&"core-008".to_string()));
    }

    #[test]
    fn r1061_an_x_cost_unit_is_never_drawn() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [SMALL], "library": ["classicplus-069", "core-008", "core-011", "core-003"] },
            "p2": { "hand": [FILLER] },
        }));
        s.play(SMALL, json!({}));
        for id in drawn_ids(&s) {
            assert_ne!(s.card(&id).def_id, "classicplus-069");
        }
    }

    #[test]
    fn r97_hidden_from_the_opponent() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [SMALL], "library": ["core-008", "core-011", "core-003"] },
            "p2": { "hand": [FILLER] },
        }));
        s.play(SMALL, json!({}));
        let text = serde_json::to_string(&s.view(PlayerId::P2)).unwrap();
        for id in drawn_ids(&s) {
            assert!(!text.contains(&id));
        }
    }

    #[test]
    fn r386_cards_reads_through_param() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [SMALL], "library": ["core-008", "core-011", "core-003", "core-086"] },
            "p2": { "hand": [FILLER] },
        }));
        step_param(s.card_mut(SMALL), "cards", 1);
        s.play(SMALL, json!({}));
        assert_eq!(drawn_ids(&s).len(), 4);
    }

    #[test]
    fn radiant_each_drawn_unit_is_radiant() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": SMALL, "radiant": true }], "library": ["core-008", "core-011", "core-003"] },
            "p2": { "hand": [FILLER] },
        }));
        s.play(SMALL, json!({}));
        for id in drawn_ids(&s) {
            assert!(s.card(&id).radiant);
        }
    }

    #[test]
    fn radiant_one_the_hand_cap_burns_stays_base() {
        crate::register_all();
        let mut hand = vec![json!({ "def": SMALL, "radiant": true })];
        hand.extend((0..9).map(|_| json!(FILLER)));
        let mut s = scenario(json!({
            "p1": { "hand": hand, "library": ["core-008", "core-011", "core-003"] },
            "p2": { "hand": [FILLER] },
        }));
        s.play(SMALL, json!({}));
        let drawn = drawn_ids(&s);
        assert_eq!(s.hand(PlayerId::P1).len(), 10);
        let burned: Vec<String> = s.pile(PlayerId::P1, "graveyard").iter().map(|card| card.id.clone()).collect();
        assert!(!burned.is_empty());
        for id in &burned {
            if drawn.contains(id) {
                assert!(!s.card(id).radiant);
            }
        }
    }
}
