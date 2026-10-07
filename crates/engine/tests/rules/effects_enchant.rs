//! Enchantments that ride a card (docs/classic-sets.md B5 E39; R443): `enchant` on a named card or a
//! scope, never twice the same, kept in every zone and through leaving the field (R78's reset leaves
//! them alone), carried by a copy and a Fuse, gone with a Transform, and shown where the card is read.
//!
//! Port of `packages/engine/test/effects-enchant.test.ts`.

use jackioh_engine::effects::{enchant, shuffle_copies_of_self, summon_copy, transform};
use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::harness::{in_hand, put, set_library, slot};
use crate::rules::fixtures::instance_data::{body, echo_bolt, instance_game};

const RETURN: Enchantment = Enchantment::ReturnAfterResolve { floor: 2 };
const CAST: Enchantment = Enchantment::CastOnDraw;
const ENEMIES: Enchantment = Enchantment::TargetEnemies;

fn game() -> GameState {
    let mut state = instance_game("enchant", None);
    state.turn = 3;
    state.active = PlayerId::P1;
    state.phase = Phase::Main;
    state
}

/// TS `run(state, effect, { self, ...hook })`: the effect applied for p1 (unless `hook` names another
/// controller) on a fresh sink over `state`; its events. Like TS's, the rng is not written back.
fn run(state: &mut GameState, effect: Effect, self_: Option<&CardInstance>, hook: HookOptions) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let controller = hook.controller.unwrap_or(PlayerId::P1);
        let mut ctx = make_context(&mut sink, self_, HookOptions { controller: Some(controller), ..hook });
        (effect.apply)(&mut ctx);
    }
    events
}

fn enchant_one(id: &str, enchantment: Enchantment) -> Effect {
    enchant(json_as(json!({ "instanceId": id, "enchantment": enchantment })))
}

/// The card as it stands in the state now (TS held the live object).
fn live<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).expect("the card is in the state")
}

/// `enchantmentsOf(card)` for the card under `id` as it stands now.
fn on(state: &GameState, id: &str) -> Vec<Enchantment> {
    enchantments_of(live(state, id)).to_vec()
}

/// `state.players.p1.units[lane - 1]?.[0]`.
fn top_unit(state: &GameState, lane: usize) -> Option<&CardInstance> {
    state.players[PlayerId::P1].units[lane - 1].as_ref().and_then(|pile| pile.first())
}

mod b5_e39_enchant_r443 {
    use super::*;

    #[test]
    fn r443_enchant_puts_an_enchantment_on_a_named_card_anywhere_never_twice_the_same_one_and_reports_nothing() {
        let mut state = game();
        let Some(deck_card) = set_library(&mut state, PlayerId::P1, &[echo_bolt.id.clone()]).into_iter().next() else {
            panic!("no card");
        };
        assert_eq!(run(&mut state, enchant_one(&deck_card.id, CAST), None, HookOptions::default()), vec![]);
        run(&mut state, enchant_one(&deck_card.id, CAST), None, HookOptions::default());
        run(&mut state, enchant_one(&deck_card.id, ENEMIES), None, HookOptions::default());
        assert_eq!(on(&state, &deck_card.id), vec![CAST, ENEMIES]);
        assert!(has_enchantment(live(&state, &deck_card.id), EnchantmentKind::TargetEnemies));
        // Two of a kind that differ are both kept, for their reader to combine.
        let card = find_instance_mut(&mut state, &deck_card.id).expect("the card is in the state");
        assert!(add_enchantment(card, &RETURN));
        assert!(add_enchantment(card, &Enchantment::ReturnAfterResolve { floor: 1 }));
        assert!(!add_enchantment(card, &RETURN));
        assert_eq!(
            enchantments_of_kind(live(&state, &deck_card.id), EnchantmentKind::ReturnAfterResolve).to_vec(),
            vec![RETURN, Enchantment::ReturnAfterResolve { floor: 1 }]
        );
    }

    #[test]
    fn r443_enchant_over_a_scope_reaches_every_card_of_it_hand_and_deck_included() {
        let mut state = game();
        let hand = in_hand(&mut state, &echo_bolt.id, PlayerId::P1, 2);
        let deck = set_library(&mut state, PlayerId::P1, &[echo_bolt.id.clone(), plain.id.clone()]);
        run(
            &mut state,
            enchant(json_as(json!({
                "scope": { "zones": ["hand", "library"], "types": ["Spell"] },
                "enchantment": CAST,
            }))),
            None,
            HookOptions::default(),
        );
        assert_eq!(hand.iter().map(|card| on(&state, &card.id)).collect::<Vec<_>>(), vec![vec![CAST], vec![CAST]]);
        assert_eq!(deck.iter().map(|card| on(&state, &card.id)).collect::<Vec<_>>(), vec![vec![CAST], vec![]]);
    }

    #[test]
    fn r443_r78_r215_an_enchantment_rides_the_card_through_every_zone_and_through_leaving_the_field() {
        let mut state = game();
        let unit = put(&mut state, &body.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        run(&mut state, enchant_one(&unit.id, RETURN), None, HookOptions::default());
        let now = live(&state, &unit.id).clone();
        move_to_zone(&mut state, &now, OffFieldZone::Hand, Default::default());
        assert_eq!(on(&state, &unit.id), vec![RETURN]);
        for zone in [OffFieldZone::Graveyard, OffFieldZone::Library, OffFieldZone::Exile] {
            let now = live(&state, &unit.id).clone();
            move_to_zone(&mut state, &now, zone, Default::default());
        }
        assert_eq!(on(&state, &unit.id), vec![RETURN]);
        // TS kept the live object after emptying the exile pile; here the copy taken just before.
        let exiled = live(&state, &unit.id).clone();
        state.players[PlayerId::P1].exile = vec![];
        assert!(place_on_field(&mut state, &exiled, &slot(PlayerId::P1, Row::Units, 2), Default::default()));
        assert_eq!(on(&state, &unit.id), vec![RETURN]);
    }

    #[test]
    fn r443_a_copy_carries_its_source_s_enchantments_a_fuse_unites_its_ingredients_a_transform_makes_a_card_without_them(
    ) {
        let mut state = game();
        let unit = put(&mut state, &body.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        run(&mut state, enchant_one(&unit.id, ENEMIES), None, HookOptions::default());
        run(
            &mut state,
            summon_copy(json_as(json!({ "of": { "of": "instance", "instanceId": unit.id } }))),
            None,
            HookOptions::default(),
        );
        let copy = top_unit(&state, 2).unwrap_or_else(|| live(&state, &unit.id));
        assert_eq!(enchantments_of(copy).to_vec(), vec![ENEMIES]);

        let spell = new_instance(&mut state, &echo_bolt.id, PlayerId::P1, Zone::Resolving { player: PlayerId::P1 });
        state.players[PlayerId::P1].resolving.push(spell.clone());
        run(&mut state, enchant_one(&spell.id, CAST), None, HookOptions::default());
        let spell_now = live(&state, &spell.id).clone();
        run(&mut state, shuffle_copies_of_self(json_as(json!({ "count": 1 }))), Some(&spell_now), HookOptions::default());
        let copies: Vec<Vec<Enchantment>> = state.players[PlayerId::P1]
            .library
            .iter()
            .filter(|card| card.def_id == echo_bolt.id)
            .map(|card| enchantments_of(card).to_vec())
            .collect();
        assert_eq!(copies, vec![vec![CAST]]);

        let other = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 3), json!({}));
        run(&mut state, enchant_one(&other.id, CAST), None, HookOptions::default());
        run(&mut state, enchant_one(&other.id, ENEMIES), None, HookOptions::default());
        {
            let other_now = live(&state, &other.id).clone();
            let unit_now = live(&state, &unit.id).clone();
            let mut events = Vec::new();
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            fuse(&mut sink, FuseArgs { ingredients: vec![other_now], target: Some(unit_now), ..Default::default() });
        }
        // In ingredient order, the target last (R77), each once.
        assert_eq!(on(&state, &unit.id), vec![CAST, ENEMIES]);
        // TS's `{ enchantments: [CAST] }` and `{}`: two instances numbered apart from the game's own ids.
        let mut numbering: u32 = 1;
        let mut enchanted = new_instance(&mut numbering, &plain.id, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        enchanted.enchantments = Some(vec![CAST]);
        let bare = new_instance(&mut numbering, &plain.id, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        assert_eq!(united_enchantments(&[enchanted, bare.clone()]), Some(vec![CAST]));
        assert_eq!(united_enchantments(&[bare.clone(), bare]), None);

        run(
            &mut state,
            transform(json_as(json!({ "instanceId": unit.id, "defId": plain.id }))),
            None,
            HookOptions::default(),
        );
        let transformed = top_unit(&state, 1).unwrap_or_else(|| live(&state, &unit.id));
        assert_eq!(enchantments_of(transformed).to_vec(), Vec::<Enchantment>::new());
    }

    #[test]
    fn r443_the_view_shows_the_enchantments_where_the_card_is_read_and_the_other_player_never_reads_a_hand_card_s() {
        let mut state = game();
        let Some(held) = in_hand(&mut state, &echo_bolt.id, PlayerId::P1, 1).into_iter().next() else {
            panic!("no card");
        };
        run(&mut state, enchant_one(&held.id, RETURN), None, HookOptions::default());
        let HandView::Cards(hand) = view_for(&state, PlayerId::P1).you.hand else {
            panic!("own hand is a list");
        };
        let shown = hand.iter().find(|card| card.instance_id == held.id).and_then(|card| card.enchantments.clone());
        assert_eq!(shown, Some(vec![RETURN]));
        let theirs = serde_json::to_string(&view_for(&state, PlayerId::P2)).expect("the view serialises");
        assert!(!theirs.contains("returnAfterResolve"));
    }
}
