//! Stinkyscape #126 Jlockeed's Permanent Procurement Lobbying (SPEC §8.10 row 126). (2) Spell, Jlockeed,
//! Epic.
//!   Base:    "For the rest of the game. / Start of turn: Add {cards|random Starship Piece|…} to your
//!            hand." — cards 1
//!   Radiant: "… Add {cards|random Radiant Starship Piece|…} to your hand. Each costs ({discount}) less."
//!            — cards 1, discount 1
//!
//! forRestOfGame leaves modifier running at start of turn (C+ #52, R458, R62, R127, R594).
//! Adds cards from Starship Piece pool (S #124, R1878000; §2.4, R65, R767000).

use jackioh_engine::effects::for_rest_of_game;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-126";

/// §10.6's data is JSON: a number carried in the entry, or the printed value when it is missing.
fn carried(ctx: &EffectContext<'_>, key: &str) -> i32 {
    match ctx.data.get(key).and_then(Value::as_i64) {
        Some(value) => value as i32,
        None => param(ctx, key),
    }
}

fn lobbying(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let cards = param(&*ctx, "cards");
            let discount = param(&*ctx, "discount");
            let numbers: IndexMap<String, i32> =
                IndexMap::from([("cards".to_string(), cards), ("discount".to_string(), discount)]);
            let face = if radiant { FaceKind::Radiant } else { FaceKind::Base };
            let label = fill_params(def_of(Some(&*ctx.state), ID), face, Some(&numbers));
            vec![for_rest_of_game(json_as(json!({
                "step": "lobbying",
                "label": label,
                "data": { "cards": cards, "discount": discount },
            })))]
        })),
        delayed: Some(hook(move |ctx| {
            let mut args = AddRandomFromCatalogArgs {
                query: Some(crate::query::starship_pieces()),
                count: Some(carried(ctx, "cards")),
                ..AddRandomFromCatalogArgs::default()
            };
            if radiant {
                args.radiant = Some(true);
                args.cost_mod = Some(-carried(ctx, "discount"));
            }
            vec![add_random_from_catalog(args)]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: lobbying(false),
        radiant: lobbying(true),
    }
}

// Stinkyscape #126 Jlockeed's Permanent Procurement Lobbying — SPEC §8.10 row 126, BUILD M12 row
// S 126 (R458, R62, R824000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const LOBBYING: &str = "stinkyscape-126";
    const FILLER: &str = "core-005";

    fn lobbied(radiant: bool, fillers: usize) -> Scenario {
        let mut hand = vec![json!({ "def": LOBBYING, "radiant": radiant })];
        hand.extend((0..fillers).map(|_| json!(FILLER)));
        let mut s = scenario(json!({
            "seed": "lobbying",
            "p1": { "hand": hand, "library": filler(8), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(8) },
        }));
        s.play(LOBBYING, json!({}));
        s
    }

    fn prints_piece(def_id: &str) -> bool {
        has_keyword(&crate::card_def(def_id).base.keywords, KeywordKind::StarshipPiece)
    }

    fn delivered(s: &Scenario) -> Vec<GameEvent> {
        s.last_events()
            .iter()
            .filter(|event| match event {
                GameEvent::AddedToHand { player: P1, def_id, .. } | GameEvent::Burned { def_id, .. } => {
                    prints_piece(def_id)
                }
                _ => false,
            })
            .cloned()
            .collect()
    }

    fn instance_of(event: &GameEvent) -> String {
        match event {
            GameEvent::AddedToHand { instance_id, .. } | GameEvent::Burned { instance_id, .. } => instance_id.clone(),
            _ => String::new(),
        }
    }

    #[test]
    fn r458_it_leaves_a_rest_of_game_modifier_badged_in_its_words_and_goes_to_the_graveyard() {
        let mut s = lobbied(false, 1);
        s.expect_in_zone(LOBBYING, "graveyard");
        let label = "For the rest of the game.\nStart of turn: Add 1 random Starship Piece to your hand.";
        let mine: Vec<String> = s.view(P1).you.modifiers.iter().map(|m| m.label.clone()).collect();
        assert_eq!(mine, vec![label.to_string()]);
        let theirs: Vec<String> = s.view(PlayerId::P2).opponent.modifiers.iter().map(|m| m.label.clone()).collect();
        assert_eq!(theirs, vec![label.to_string()]);
    }

    #[test]
    fn r62_each_of_your_turns_starts_with_a_random_starship_piece_before_the_draw() {
        let mut s = lobbied(false, 1);
        for _turn in 0..3 {
            s.end_turn().end_turn();
            let pieces = delivered(&s);
            assert_eq!(pieces.len(), 1, "{pieces:?}");
            assert_eq!(s.card(instance_of(&pieces[0])).zone.z(), ZoneName::Hand);
            let events = s.last_events();
            let piece = events.iter().position(|event| *event == pieces[0]);
            let drawn = events.iter().rposition(|event| matches!(event, GameEvent::Drawn { .. }));
            assert!(piece < drawn, "{events:?}");
        }
    }

    #[test]
    fn the_opponents_turns_add_none() {
        let mut s = lobbied(false, 1);
        s.end_turn();
        assert_eq!(s.state().active, PlayerId::P2);
        assert!(delivered(&s).is_empty());
    }

    #[test]
    fn s2_4_a_full_hand_burns_the_piece() {
        let mut s = lobbied(false, 10);
        s.end_turn().end_turn();
        let burned = delivered(&s)
            .into_iter()
            .filter(|event| matches!(event, GameEvent::Burned { .. }))
            .count();
        assert_eq!(burned, 1);
    }

    #[test]
    fn r594_r386_cards_is_read_as_it_resolves_and_carried() {
        let mut s = scenario(json!({
            "seed": "lobbying-param",
            "p1": { "hand": [LOBBYING, FILLER], "library": filler(8), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(8) },
        }));
        step_param(s.card_mut(LOBBYING), "cards", 1);
        s.play(LOBBYING, json!({}));
        s.end_turn().end_turn();
        assert_eq!(delivered(&s).len(), 2);
        s.end_turn().end_turn();
        assert_eq!(delivered(&s).len(), 2);
    }

    #[test]
    fn radiant_each_piece_is_radiant_and_costs_1_less_never_below_0() {
        let mut s = lobbied(true, 1);
        for _turn in 0..4 {
            s.end_turn().end_turn();
            let pieces = delivered(&s);
            assert_eq!(pieces.len(), 1, "{pieces:?}");
            let card = s.card(instance_of(&pieces[0])).clone();
            assert!(card.radiant);
            assert_eq!(card.cost_mod, -1);
            let printed = query_cost(&crate::card_def(&card.def_id));
            assert_eq!(effective_cost(s.state(), &card, Default::default()), (printed - 1).max(0));
        }
    }

    #[test]
    fn radiant_r824000_the_discount_reads_through_param() {
        let mut s = scenario(json!({
            "seed": "lobbying-discount",
            "p1": { "hand": [{ "def": LOBBYING, "radiant": true }, FILLER], "library": filler(8), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(8) },
        }));
        step_param(s.card_mut(LOBBYING), "discount", 1);
        s.play(LOBBYING, json!({}));
        s.end_turn().end_turn();
        let pieces = delivered(&s);
        assert_eq!(pieces.len(), 1);
        assert_eq!(s.card(instance_of(&pieces[0])).cost_mod, -2);
    }
}
