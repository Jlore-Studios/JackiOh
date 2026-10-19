// Meditative+ batch 03 (issue #1181): the engine systems M+ #4 Supply Drop, #22 Mountaintop Garden and
// #23 Enormous Elderberry are written with, each through a fixture script (`fixtures/mplus_b03.rs`)
// before a card uses it: R1181000's effect made "in N turns" with its countdown badge (`delay`'s
// `turns` and `label`), R1181002's aura over embiggen prices (the `embiggenGarden` flag) and
// R1181003's price for the Embiggened version alone (`add_to_hand`'s `embiggenCostOverride`).

use jackioh_engine::effects;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::mplus_b03::{
    CASTER, ELDER, ELDER_COST, GARDEN, GARDEN_PREMIUM, GARDEN_PREMIUM_RADIANT, LATER, LATER_DAMAGE,
    LATER_LABEL, LATER_TURNS, SWELL, SWELL_BASE, SWELL_EMBIGGEN, register_b03_fixtures,
};

/// `p1`'s turn 9, its main phase (or `active`'s); each side's setup as given, with plain cards to
/// draw and to hold so no turn ends by itself (§2.5).
fn board_on(seed: &str, active: PlayerId, p1: Value, p2: Value) -> Scenario {
    register_b03_fixtures();
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([plain.id, plain.id, plain.id, plain.id, plain.id, plain.id]);
        }
        if side.get("hand").is_none() {
            side["hand"] = json!([plain.id]);
        }
    }
    scenario(json!({ "seed": seed, "active": active.as_str(), "p1": p1, "p2": p2 }))
}

fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    board_on(seed, P1, p1, p2)
}

/// The hits the delayed effect dealt in these events, as (target, amount).
fn hits(events: &[GameEvent]) -> Vec<(String, i32)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Damage {
                target_id, amount, ..
            } => Some((target_id.clone(), *amount)),
            _ => None,
        })
        .collect()
}

/// The labels of the badges `viewer` sees on `side`'s seat.
fn badges(s: &Scenario, viewer: PlayerId, side: PlayerId) -> Vec<String> {
    let view = s.view(viewer);
    let seat = if viewer == side { view.you } else { view.opponent };
    seat.modifiers.into_iter().map(|badge| badge.label).collect()
}

fn countdown(left: &str) -> String {
    format!("{LATER_LABEL} {left}")
}

/// p1's hand card `id` as p1's view shows it.
fn shown(s: &Scenario, id: &str) -> CardView {
    match s.view(P1).you.hand {
        HandView::Cards(cards) => cards
            .into_iter()
            .find(|card| card.instance_id == id)
            .expect("p1 sees its own hand card"),
        HandView::Count { .. } => panic!("p1's own hand is shown as cards"),
    }
}

/// The `embiggen` flags `legalActions` offers the card's plays at.
fn offered(s: &Scenario, id: &str) -> Vec<Option<bool>> {
    let mut flags: Vec<Option<bool>> = legal_actions(s.state(), P1)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::Play {
                instance_id,
                embiggen,
                face_down: None,
                ..
            } if instance_id == id => Some(embiggen),
            _ => None,
        })
        .collect();
    flags.sort();
    flags.dedup();
    flags
}

/// The last play's `cardPlayed`: what it paid and the version it was played as.
fn played(s: &Scenario) -> (i32, Option<bool>) {
    s.last_events()
        .iter()
        .find_map(|event| match event {
            GameEvent::CardPlayed {
                cost_paid,
                embiggened,
                ..
            } => Some((*cost_paid, *embiggened)),
            _ => None,
        })
        .expect("the play is announced")
}

fn swells_in_hand(s: &Scenario) -> Vec<String> {
    s.hand(P1)
        .into_iter()
        .filter(|card| card.def_id == SWELL.id)
        .map(|card| card.id)
        .collect()
}

mod r1181000_in_n_turns_counts_its_owners_turns_and_shows_a_countdown {
    use super::*;

    #[test]
    fn r1181000_a_three_turn_delay_resolves_at_its_owners_third_turn_start_before_the_draw() {
        let mut s = board(
            "r1181000-three",
            json!({ "hand": [LATER.id, plain.id], "health": 20 }),
            json!({ "armor": 3, "field": [{ "def": plain.id, "lane": 1 }] }),
        );
        let theirs = s.unit(P2, 1).expect("p2's unit").id;
        s.play(LATER.id.as_str(), json!({}));
        for _ in 0..4 {
            s.end_turn();
        }
        assert!(
            hits(s.events()).is_empty(),
            "two of p1's turns have started: nothing yet"
        );

        s.end_turn().end_turn();
        let events = s.last_events().to_vec();
        assert_eq!(
            hits(&events),
            vec![
                (theirs.clone(), LATER_DAMAGE),
                ("hero-p2".to_string(), LATER_DAMAGE)
            ],
            "each enemy, the Unit then the hero"
        );
        let first_hit = events
            .iter()
            .position(|event| matches!(event, GameEvent::Damage { .. }))
            .expect("a hit");
        let draw = events
            .iter()
            .position(|event| matches!(event, GameEvent::Drawn { player: P1, .. }))
            .expect("p1's turn draw");
        assert!(first_hit < draw, "R62: the delayed effect before the draw");
        assert_eq!(
            s.state().players.p2.hero.health,
            30 - LATER_DAMAGE,
            "Pierce: through the Armor"
        );
        assert_eq!(s.state().players.p2.hero.armor, 3, "the Armor untouched");
        assert_eq!(
            s.state().players.p1.hero.health,
            20 + 2 * LATER_DAMAGE,
            "Lifesteal, once per hit"
        );
        assert!(s.state().delayed.is_empty());
    }

    #[test]
    fn r1181000_an_extra_turn_counts() {
        let mut s = board(
            "r1181000-extra",
            json!({ "hand": [LATER.id, plain.id] }),
            json!({}),
        );
        s.play(LATER.id.as_str(), json!({}));
        s.state_mut().players.p1.extra_turns = Some(1);
        for _ in 0..4 {
            s.end_turn();
        }
        assert!(hits(s.events()).is_empty());
        s.end_turn();
        assert_eq!(s.state().active, P1);
        assert_eq!(
            hits(s.last_events()),
            vec![("hero-p2".to_string(), LATER_DAMAGE)],
            "the extra turn was the first of the three"
        );
    }

    #[test]
    fn r1181000_made_on_the_opponents_turn_it_counts_from_the_owners_next() {
        let mut s = board_on("r1181000-their-turn", P2, json!({}), json!({}));
        // A cast for p1 on p2's turn (R70), as an effect casts it.
        let mut rng = create_rng(&s.state().seed, s.state().rng_cursor);
        let mut events: Vec<GameEvent> = Vec::new();
        {
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            {
                let mut ctx = make_context(
                    &mut sink,
                    None,
                    HookOptions {
                        controller: Some(P1),
                        ..HookOptions::default()
                    },
                );
                apply_effects(
                    &[effects::cast_new(json_as(json!({ "def": LATER.id })))],
                    &mut ctx,
                );
            }
            settle(&mut sink, SettleOptions::default());
        }
        s.state_mut().rng_cursor = rng.cursor();
        assert_eq!(s.state().delayed.len(), 1, "it waits");
        for _ in 0..4 {
            s.end_turn();
        }
        assert!(
            hits(s.events()).is_empty(),
            "not at p1's second turn start (the 3rd turn end)"
        );
        s.end_turn();
        assert_eq!(s.state().active, P1);
        assert_eq!(hits(s.last_events()), vec![("hero-p2".to_string(), LATER_DAMAGE)]);
    }

    #[test]
    fn r1181000_the_countdown_is_a_badge_on_both_seats() {
        let mut s = board(
            "r1181000-badge",
            json!({ "hand": [LATER.id, plain.id] }),
            json!({}),
        );
        assert!(badges(&s, P1, P1).is_empty());
        s.play(LATER.id.as_str(), json!({}));
        let id = s.state().delayed[0].id.clone();
        assert!(s.last_events().iter().any(|event| matches!(
            event,
            GameEvent::ModifierChanged { player: P1, modifier_id, added: true } if *modifier_id == id
        )));
        let three = countdown(&format!("in {LATER_TURNS} turns"));
        assert_eq!(badges(&s, P1, P1), vec![three.clone()]);
        assert_eq!(
            badges(&s, P2, P1),
            vec![three],
            "public: the opponent sees it too (R169)"
        );
        assert!(badges(&s, P2, P2).is_empty());

        s.end_turn().end_turn();
        assert_eq!(badges(&s, P1, P1), vec![countdown("in 2 turns")]);
        s.end_turn().end_turn();
        assert_eq!(badges(&s, P2, P1), vec![countdown("in 1 turn")]);
        s.end_turn().end_turn();
        assert!(badges(&s, P1, P1).is_empty(), "gone once it resolved");
        assert!(badges(&s, P2, P1).is_empty());
        assert!(s.last_events().iter().any(|event| matches!(
            event,
            GameEvent::ModifierChanged { player: P1, modifier_id, added: false } if *modifier_id == id
        )));
    }

    #[test]
    fn r1181000_a_game_with_no_counted_delay_serializes_no_new_field() {
        let mut s = board(
            "r1181000-absent",
            json!({ "hand": [SWELL.id, SWELL.id, plain.id], "mana": 10 }),
            json!({}),
        );
        let ids = swells_in_hand(&s);
        s.play(ids[0].as_str(), json!({ "zone": 1 }));
        s.play(ids[1].as_str(), json!({ "zone": 2, "embiggen": true }));
        s.end_turn().end_turn();
        let text = serde_json::to_string(s.state()).expect("a state serialises");
        assert!(!text.contains("dueOwnTurn"), "D9: no counted delay, no field");
        assert!(!text.contains("embiggenCostOverride"));
        assert!(!text.contains("\"garden\""));
    }
}

mod r1181002_mountaintop_garden_plays_embiggen_cards_embiggened_and_its_dearer_price_radiant {
    use super::*;

    #[test]
    fn r1181002_the_normal_price_plays_embiggened_and_the_dearer_price_is_radiant_for_normal_plus_premium() {
        let mut s = board(
            "r1181002-prices",
            json!({ "hand": [SWELL.id, SWELL.id, plain.id], "backrow": [GARDEN.id], "mana": 10 }),
            json!({}),
        );
        let ids = swells_in_hand(&s);
        let card = shown(&s, &ids[0]);
        assert_eq!(card.cost, SWELL_BASE);
        assert_eq!(card.embiggen_cost, Some(SWELL_BASE + GARDEN_PREMIUM));
        assert_eq!(offered(&s, &ids[0]), vec![Some(false), Some(true)]);

        s.play(ids[0].as_str(), json!({ "zone": 2 }));
        assert_eq!(
            played(&s),
            (SWELL_BASE, Some(true)),
            "the normal price, the Embiggened version"
        );
        assert!(!s.card(ids[0].as_str()).radiant);
        assert_eq!(s.card(ids[0].as_str()).embiggened, Some(true));

        s.play(ids[1].as_str(), json!({ "zone": 3, "embiggen": true }));
        assert_eq!(played(&s), (SWELL_BASE + GARDEN_PREMIUM, Some(true)));
        assert!(
            s.card(ids[1].as_str()).radiant,
            "the dearer price plays it Radiant"
        );
        assert_eq!(s.card(ids[1].as_str()).embiggened, Some(true));
        assert!(s.last_events().iter().any(|event| matches!(
            event,
            GameEvent::RadiantSet { instance_id, .. } if *instance_id == ids[1]
        )));
    }

    #[test]
    fn r1181002_a_radiant_card_offers_only_its_normal_price() {
        let mut s = board(
            "r1181002-radiant",
            json!({
                "hand": [{ "def": SWELL.id, "radiant": true }, plain.id],
                "backrow": [GARDEN.id],
                "mana": 10,
            }),
            json!({}),
        );
        let id = swells_in_hand(&s)[0].clone();
        assert_eq!(offered(&s, &id), vec![Some(false)]);
        assert_eq!(shown(&s, &id).embiggen_cost, None, "one price, one badge");
        s.expect_refused_with(
            |s| s.play(id.as_str(), json!({ "zone": 2, "embiggen": true })),
            "has only its normal price now",
        );
        s.play(id.as_str(), json!({ "zone": 2 }));
        assert_eq!(played(&s), (SWELL_BASE, Some(true)));
        assert!(s.card(id.as_str()).radiant);
    }

    #[test]
    fn r1181002_the_opponents_garden_reaches_your_cards_too() {
        let mut s = board(
            "r1181002-theirs",
            json!({ "hand": [SWELL.id, SWELL.id, plain.id], "mana": 10 }),
            json!({ "backrow": [GARDEN.id] }),
        );
        let ids = swells_in_hand(&s);
        assert_eq!(
            shown(&s, &ids[0]).embiggen_cost,
            Some(SWELL_BASE + GARDEN_PREMIUM)
        );
        s.play(ids[0].as_str(), json!({ "zone": 1 }));
        assert_eq!(played(&s), (SWELL_BASE, Some(true)));
        s.play(ids[1].as_str(), json!({ "zone": 2, "embiggen": true }));
        assert_eq!(played(&s), (SWELL_BASE + GARDEN_PREMIUM, Some(true)));
        assert!(s.card(ids[1].as_str()).radiant);
    }

    #[test]
    fn r1181002_two_gardens_apply_the_least_premium() {
        let s = board(
            "r1181002-two",
            json!({ "hand": [SWELL.id, plain.id], "backrow": [GARDEN.id] }),
            json!({ "backrow": [{ "def": GARDEN.id, "radiant": true }] }),
        );
        let id = swells_in_hand(&s)[0].clone();
        assert_eq!(
            shown(&s, &id).embiggen_cost,
            Some(SWELL_BASE + GARDEN_PREMIUM_RADIANT)
        );
        assert_eq!(SWELL_BASE + GARDEN_PREMIUM_RADIANT, 3);
    }

    #[test]
    fn r1181002_cost_modifiers_apply_to_both_prices() {
        let mut s = board(
            "r1181002-modifiers",
            json!({ "hand": [{ "def": SWELL.id, "costMod": -1 }, plain.id], "backrow": [GARDEN.id], "mana": 10 }),
            json!({}),
        );
        let id = swells_in_hand(&s)[0].clone();
        let card = shown(&s, &id);
        assert_eq!(
            (card.cost, card.embiggen_cost),
            (1, Some(3)),
            "R65: after the Garden's price"
        );
        s.play(id.as_str(), json!({ "zone": 2, "embiggen": true }));
        assert_eq!(played(&s), (3, Some(true)));
    }

    #[test]
    fn r1181002_a_cast_is_unchanged_r545() {
        let mut s = board(
            "r1181002-cast",
            json!({ "hand": [CASTER.id, plain.id], "backrow": [GARDEN.id] }),
            json!({}),
        );
        s.play(CASTER.id.as_str(), json!({}));
        let cast = s.backrow(P1, 2).expect("the cast swell took the next zone");
        assert_eq!(cast.def_id, SWELL.id);
        assert_ne!(cast.embiggened, Some(true), "a cast resolves at its base (R545)");
        assert!(!cast.radiant);
    }

    #[test]
    fn r1181002_a_card_set_face_down_keeps_its_printed_prices_r783005() {
        let mut s = board(
            "r1181002-face-down",
            json!({ "hand": [SWELL.id, SWELL.id, plain.id], "backrow": [GARDEN.id], "mana": 10 }),
            json!({}),
        );
        let ids = swells_in_hand(&s);
        for id in &ids {
            s.card_mut(id).granted_keywords.push(Keyword::Cloaked);
        }
        let mana = |s: &Scenario| s.state().players.p1.mana.current;

        let before = mana(&s);
        s.play(ids[0].as_str(), json!({ "zone": 2, "faceDown": "cloaked" }));
        assert_eq!(before - mana(&s), SWELL_BASE);
        let set = s.backrow(P1, 2).expect("the set swell");
        assert_eq!(set.embiggened, Some(false), "set as its normal version");

        let before = mana(&s);
        s.play(
            ids[1].as_str(),
            json!({ "zone": 3, "faceDown": "cloaked", "embiggen": true }),
        );
        assert_eq!(before - mana(&s), SWELL_EMBIGGEN, "the printed Embiggened price");
        let set = s.backrow(P1, 3).expect("the set swell");
        assert!(!set.radiant, "a set card is not played Radiant");
    }

    #[test]
    fn r1181002_without_a_garden_the_prices_are_printed() {
        let s = board(
            "r1181002-none",
            json!({ "hand": [{ "def": SWELL.id, "radiant": true }, plain.id] }),
            json!({}),
        );
        let id = swells_in_hand(&s)[0].clone();
        assert_eq!(offered(&s, &id), vec![Some(false), Some(true)]);
        assert_eq!(shown(&s, &id).embiggen_cost, Some(SWELL_EMBIGGEN));
    }
}

mod r1181003_a_price_for_the_embiggened_version_alone {
    use super::*;

    #[test]
    fn r1181003_the_embiggened_price_alone_is_overridden() {
        let mut s = board(
            "r1181003-elder",
            json!({ "hand": [ELDER.id, ELDER.id, plain.id], "mana": 10 }),
            json!({}),
        );
        s.play(ELDER.id.as_str(), json!({}));
        s.play(ELDER.id.as_str(), json!({}));
        let ids = swells_in_hand(&s);
        assert_eq!(ids.len(), 2);
        assert_eq!(s.card(ids[0].as_str()).embiggen_cost_override, Some(ELDER_COST));
        let card = shown(&s, &ids[0]);
        assert_eq!((card.cost, card.embiggen_cost), (SWELL_BASE, Some(ELDER_COST)));
        assert_eq!(offered(&s, &ids[0]), vec![Some(false), Some(true)]);

        s.play(ids[0].as_str(), json!({ "zone": 1 }));
        assert_eq!(
            played(&s),
            (SWELL_BASE, Some(false)),
            "the normal version keeps its price"
        );
        s.play(ids[1].as_str(), json!({ "zone": 2, "embiggen": true }));
        assert_eq!(played(&s), (ELDER_COST, Some(true)));
    }

    #[test]
    fn r1181003_a_cost_override_sets_the_normal_price_only() {
        let mut s = board(
            "r1181003-override",
            json!({ "hand": [{ "def": SWELL.id, "costOverride": 1 }, plain.id], "mana": 10 }),
            json!({}),
        );
        let id = swells_in_hand(&s)[0].clone();
        s.card_mut(id.as_str()).embiggen_cost_override = Some(ELDER_COST);
        let card = shown(&s, &id);
        assert_eq!((card.cost, card.embiggen_cost), (1, Some(ELDER_COST)));
        s.play(id.as_str(), json!({ "zone": 1, "embiggen": true }));
        assert_eq!(played(&s), (ELDER_COST, Some(true)));
    }

    #[test]
    fn r1181003_reset_price_takes_it_off_r766() {
        let mut s = board(
            "r1181003-reset",
            json!({ "hand": [ELDER.id, plain.id] }),
            json!({}),
        );
        s.play(ELDER.id.as_str(), json!({}));
        let id = swells_in_hand(&s)[0].clone();
        let mut card = s.card(id.as_str()).clone();
        assert_eq!(card.embiggen_cost_override, Some(ELDER_COST));
        reset_price(&mut card);
        assert_eq!(card.embiggen_cost_override, None);
        assert_eq!(card.cost_override, None);
    }

    #[test]
    fn r1181003_under_the_garden_it_prices_the_dearer_choice() {
        let mut s = board(
            "r1181003-garden",
            json!({ "hand": [ELDER.id, plain.id], "backrow": [GARDEN.id] }),
            json!({}),
        );
        s.play(ELDER.id.as_str(), json!({}));
        let id = swells_in_hand(&s)[0].clone();
        let card = shown(&s, &id);
        assert_eq!((card.cost, card.embiggen_cost), (SWELL_BASE, Some(ELDER_COST)));
        s.play(id.as_str(), json!({ "zone": 2, "embiggen": true }));
        assert_eq!(played(&s), (ELDER_COST, Some(true)));
        assert!(s.card(id.as_str()).radiant, "Radiant and Embiggened for (0)");
    }
}
