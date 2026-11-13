// The verbs Stinkyscape+ S22 (issue #3569) added, each through a fixture script before a card uses it:
// every card Radiant for the rest of the game (R3569000), a card scope that reaches graveyards and
// exile (R3569001), a fusion of the whole board into a fresh card (R3569002) and a loss of max mana
// for the rest of the game (R3569008). The cards' own tests play them in the real scripts; these pin
// the verbs themselves, so the engine never imports `crates/cards`. Ids are `s22-`.

use jackioh_engine::effects::{self, FuseBoardIntoArgs, LoseMaxManaArgs};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{apply_now, setup_catalog};

const RADIANT_WORLD: &str = "s22-radiant-world";
const UPGRADE_PILES: &str = "s22-upgrade-piles";
const DEGRADE_PILES: &str = "s22-degrade-piles";
const BOARD_FUSE: &str = "s22-board-fuse";
/// The fresh card the board fuses into: a 10/10 that spells cannot touch, as Alnilam is.
const KEPT: &str = "s22-kept";
/// A 1/1 and a 2/3 Unit, so the fusion's sum names its ingredients.
const SMALL: &str = "s22-small";
const MEDIUM: &str = "s22-medium";
/// A Unit no Spell reaches.
const IMMUNE: &str = "s22-immune";
const FILLER: &str = "s22-filler";

fn card(index: u32, id: &str, type_: &str, stats: Option<(i32, i32)>, keywords: Value) -> CardDef {
    let face = match stats {
        Some((attack, health)) => {
            json!({ "attack": attack, "health": health, "keywords": keywords, "text": id })
        }
        None => json!({ "keywords": keywords, "text": id }),
    };
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (S22 fixture)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face.clone(),
        "radiant": face,
    }))
}

fn spell_script(effects_of: impl Fn() -> Vec<Effect> + Send + Sync + 'static) -> CardScripts {
    let script = Script {
        cry: Some(hook(move |_ctx| effects_of())),
        ..Script::default()
    };
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn register() {
    setup_catalog();
    let mut catalog: CardDefs = registered_catalog().clone();
    for def in [
        card(356900, RADIANT_WORLD, "Spell", None, json!([])),
        card(356901, UPGRADE_PILES, "Spell", None, json!([])),
        card(356902, BOARD_FUSE, "Spell", None, json!([])),
        card(
            356903,
            KEPT,
            "Unit",
            Some((10, 10)),
            json!([{ "kind": "Immune to Spells" }]),
        ),
        card(356904, SMALL, "Unit", Some((1, 1)), json!([])),
        card(356905, MEDIUM, "Unit", Some((2, 3)), json!([])),
        card(
            356906,
            IMMUNE,
            "Unit",
            Some((1, 1)),
            json!([{ "kind": "Immune to Spells" }]),
        ),
        card(356907, FILLER, "Unit", Some((1, 1)), json!([])),
        card(356908, DEGRADE_PILES, "Spell", None, json!([])),
    ] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts: IndexMap<String, CardScripts> = registered_scripts().clone();
    scripts.insert(
        RADIANT_WORLD.to_string(),
        spell_script(|| vec![effects::make_all_radiant()]),
    );
    scripts.insert(
        UPGRADE_PILES.to_string(),
        spell_script(|| {
            vec![effects::upgrade(json_as(json!({
                "scope": { "side": "self", "zones": ["graveyard", "exile"] },
                "times": 1,
            })))]
        }),
    );
    scripts.insert(
        DEGRADE_PILES.to_string(),
        spell_script(|| {
            vec![effects::degrade(json_as(json!({
                "scope": { "side": "self", "zones": ["graveyard", "exile"] },
                "times": 1,
            })))]
        }),
    );
    scripts.insert(
        BOARD_FUSE.to_string(),
        spell_script(|| {
            vec![effects::fuse_board_into(FuseBoardIntoArgs {
                def_id: KEPT.to_string(),
                radiant: None,
            })]
        }),
    );
    register_scripts(scripts);
}

fn board(p1: Value, p2: Value) -> Scenario {
    board_seeded("s22", p1, p2)
}

fn board_seeded(seed: &str, p1: Value, p2: Value) -> Scenario {
    register();
    let mut s = scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

/// Every card of `player` in every pile and lane, with where `scenario` found it.
fn every_card(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
    let mut cards: Vec<CardInstance> = Vec::new();
    for pile in ["hand", "library", "graveyard", "exile"] {
        cards.extend(s.pile(player, pile));
    }
    for lane in 1..=5 {
        cards.extend(s.unit(player, lane));
        cards.extend(s.backrow(player, lane));
    }
    cards
}

/// Whether `card` is a fusion whose first ingredient, the kept card, is `KEPT` (R77, R179).
fn leads_with_kept(s: &Scenario, card: &CardInstance) -> bool {
    jackioh_engine::catalog::fused_id_parts(Some(s.state()), &card.def_id)
        .is_some_and(|parts| parts.first().map(String::as_str) == Some(KEPT))
}

fn units_of(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
    (1..=5).filter_map(|lane| s.unit(player, lane)).collect()
}

mod r3569000_every_card_radiant {
    use super::*;

    #[test]
    fn r3569000_every_pile_turns_radiant_and_a_card_made_later_too() {
        let mut s = board(
            json!({
                "hand": [FILLER, FILLER],
                "library": [FILLER, FILLER, FILLER],
                "field": [SMALL],
                "graveyard": [FILLER],
                "exile": [FILLER],
            }),
            json!({ "hand": [FILLER], "library": [FILLER, FILLER], "field": [MEDIUM], "exile": [FILLER] }),
        );
        for player in [P1, P2] {
            assert!(
                every_card(&s, player).iter().all(|card| !card.radiant),
                "none is Radiant yet"
            );
        }
        apply_now(s.state_mut(), effects::make_all_radiant());
        assert_eq!(s.state().radiant_world, Some(true));
        for player in [P1, P2] {
            let cards = every_card(&s, player);
            assert!(!cards.is_empty());
            assert!(
                cards.iter().all(|card| card.radiant),
                "every card of {player:?} is Radiant"
            );
        }
        // A card made after the cast turns Radiant at the next settled check, in a hand or a library.
        apply_now(
            s.state_mut(),
            effects::add_to_hand(json_as(json!({ "defId": FILLER, "player": "self" }))),
        );
        let made: Vec<CardInstance> = s.hand(P1);
        assert_eq!(made.len(), 3);
        assert!(made.iter().all(|card| card.radiant), "the new card is Radiant");
    }

    #[test]
    fn r3569000_a_game_without_the_cast_has_no_flag_and_no_card_turns_radiant() {
        let mut s = board(json!({ "hand": [FILLER], "field": [SMALL] }), json!({}));
        apply_now(s.state_mut(), effects::gain_mana(json_as(json!({ "amount": 0 }))));
        assert_eq!(s.state().radiant_world, None);
        assert!(every_card(&s, P1).iter().all(|card| !card.radiant));
    }

    #[test]
    fn r3569000_a_hidden_pile_is_cued_whole() {
        // One Radiant card in a library of four: the sweep cues all four, so the cues count nothing.
        let mut s = board(
            json!({
                "hand": [RADIANT_WORLD],
                "library": [FILLER, { "def": FILLER, "radiant": true }, FILLER, FILLER],
            }),
            json!({}),
        );
        s.play(RADIANT_WORLD, json!({}));
        let cued_in_library = s
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    GameEvent::RadiantSet { zone: Zone::Library { player }, .. } if *player == P1
                )
            })
            .count();
        assert_eq!(cued_in_library, 4);
        // The public field cues the cards it changes only.
        assert!(s.pile(P1, "library").iter().all(|card| card.radiant));
    }
}

mod r3569001_card_scope_piles {
    use super::*;

    #[test]
    fn r3569001_a_scope_reaches_graveyard_and_exile() {
        let mut s = board(
            json!({
                "hand": [UPGRADE_PILES],
                "graveyard": [FILLER, FILLER],
                "exile": [FILLER],
                "library": [FILLER, FILLER],
            }),
            json!({ "graveyard": [FILLER] }),
        );
        let reached: Vec<String> = s
            .pile(P1, "graveyard")
            .into_iter()
            .chain(s.pile(P1, "exile"))
            .map(|card| card.id)
            .collect();
        assert_eq!(reached.len(), 3);
        s.play(UPGRADE_PILES, json!({}));
        let upgraded: Vec<String> = s
            .events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Upgraded { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        // The spell itself lands in the graveyard after it resolves; it was not there to be reached.
        for id in &reached {
            assert!(upgraded.contains(id), "{id} was Buffed: {upgraded:?}");
        }
        assert_eq!(
            upgraded.len(),
            3,
            "one application each, and nothing of the library or the enemy"
        );
    }
}

/// R3596001, R766: a card in a graveyard or an exile pile is its printed card, its price included, so a
/// Buff that reaches those piles (S+ Tears of the Kwzki Tree's, R3569001) never takes the cost step,
/// though a (0) card's Degrade would have one toward (4) anywhere else.
mod r3596001_no_price_in_a_pile {
    use super::*;

    #[test]
    fn r3596001_a_degrade_of_the_piles_moves_no_price() {
        let mut changed = 0;
        for n in 0..24 {
            let mut s = board_seeded(
                &format!("r3596001-{n}"),
                json!({
                    "hand": [DEGRADE_PILES],
                    "graveyard": [FILLER, FILLER, FILLER],
                    "exile": [FILLER],
                }),
                json!({}),
            );
            s.play(DEGRADE_PILES, json!({}));
            for event in s.events() {
                if let GameEvent::Degraded {
                    instance_id, change, ..
                } = event
                {
                    assert!(
                        !matches!(change, TuningChange::Cost { .. }),
                        "seed {n}: {instance_id} took a price in a pile"
                    );
                    changed += 1;
                }
            }
            for card in s.pile(P1, "graveyard").into_iter().chain(s.pile(P1, "exile")) {
                assert_eq!(card.cost_mod, 0, "seed {n}: {} holds a price", card.id);
            }
        }
        assert!(changed >= 24 * 4, "every pile card was Degraded: {changed}");
    }
}

mod r3569002_board_fuse {
    use super::*;

    #[test]
    fn r3569002_every_unit_fuses_into_a_fresh_card_on_the_casters_side() {
        let mut s = board(
            json!({ "hand": [BOARD_FUSE], "field": [SMALL, MEDIUM] }),
            json!({ "field": [SMALL, MEDIUM] }),
        );
        s.play(BOARD_FUSE, json!({}));
        assert!(units_of(&s, P2).is_empty(), "the enemy's Units are gone");
        let mine = units_of(&s, P1);
        assert_eq!(mine.len(), 1, "one fused card stands on the caster's side");
        let fused = &mine[0];
        assert!(
            leads_with_kept(&s, fused),
            "the kept card leads the fusion: {}",
            fused.def_id
        );
        let view = s.stats(fused.id.as_str());
        assert_eq!(
            (view.attack, view.health),
            (10 + 1 + 2 + 1 + 2, 10 + 1 + 3 + 1 + 3)
        );
        assert!(
            s.events()
                .iter()
                .any(|event| matches!(event, GameEvent::Fused { .. })),
            "a fused event names the ingredients"
        );
        // R77, R86: nothing died and nothing was destroyed.
        assert!(
            s.pile(P1, "graveyard")
                .iter()
                .all(|card| card.def_id == BOARD_FUSE)
        );
        assert!(s.pile(P2, "graveyard").is_empty());
    }

    #[test]
    fn r3569002_an_immune_or_dormant_unit_stays() {
        let mut s = board(
            json!({
                "hand": [BOARD_FUSE],
                "field": [IMMUNE, { "def": MEDIUM, "slumber": {} }, SMALL],
            }),
            json!({ "field": [IMMUNE] }),
        );
        s.play(BOARD_FUSE, json!({}));
        let mine = units_of(&s, P1);
        // The immune Unit, the Dormant one and the fresh card (which took the freed zone, with the
        // small Unit fused into it).
        assert_eq!(mine.len(), 3, "{mine:?}");
        assert_eq!(mine.iter().filter(|card| card.def_id == IMMUNE).count(), 1);
        assert_eq!(mine.iter().filter(|card| card.def_id == MEDIUM).count(), 1);
        assert_eq!(units_of(&s, P2).len(), 1, "the enemy's immune Unit stays");
        let fused: Vec<&CardInstance> = mine.iter().filter(|card| leads_with_kept(&s, card)).collect();
        assert_eq!(fused.len(), 1, "{mine:?}");
        assert_ne!(fused[0].def_id, KEPT, "the small Unit went into it");
    }

    #[test]
    fn r3569002_with_no_unit_the_card_is_summoned_alone() {
        let mut s = board(json!({ "hand": [BOARD_FUSE] }), json!({}));
        s.play(BOARD_FUSE, json!({}));
        let mine = units_of(&s, P1);
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].def_id, KEPT);
        assert!(
            !s.events()
                .iter()
                .any(|event| matches!(event, GameEvent::Fused { .. }))
        );
    }

    #[test]
    fn r3569002_a_full_board_of_the_casters_own_makes_room() {
        let mut s = board(
            json!({ "hand": [BOARD_FUSE], "field": [SMALL, SMALL, SMALL, SMALL, SMALL] }),
            json!({}),
        );
        s.play(BOARD_FUSE, json!({}));
        let mine = units_of(&s, P1);
        assert_eq!(mine.len(), 1, "the five Units fused into one");
        assert!(leads_with_kept(&s, &mine[0]), "{}", mine[0].def_id);
    }

    #[test]
    fn r3569002_no_room_fuses_nothing() {
        // Five Units no Spell reaches fill the caster's row: no Unit of the caster's goes, so the fresh
        // card has no zone, and the enemy's Unit, which would have been an ingredient, stays.
        let mut s = board(
            json!({ "hand": [BOARD_FUSE], "field": [IMMUNE, IMMUNE, IMMUNE, IMMUNE, IMMUNE] }),
            json!({ "field": [SMALL] }),
        );
        s.play(BOARD_FUSE, json!({}));
        assert_eq!(units_of(&s, P1).len(), 5);
        assert!(units_of(&s, P1).iter().all(|card| card.def_id == IMMUNE));
        assert_eq!(units_of(&s, P2).len(), 1, "the enemy's Unit is still there");
        assert!(
            !s.events()
                .iter()
                .any(|event| matches!(event, GameEvent::Fused { .. }))
        );
    }
}

mod r3569008_lose_max_mana {
    use super::*;

    fn lose(s: &mut Scenario, amount: i32) {
        apply_now(
            s.state_mut(),
            effects::lose_max_mana(LoseMaxManaArgs { amount, player: None }),
        );
    }

    fn mana_of(s: &Scenario) -> (i32, i32) {
        let mana = s.state().players[P1].mana;
        (mana.current, mana.max)
    }

    fn at(s: &mut Scenario, current: i32, max: i32) {
        let side = &mut s.state_mut().players[P1];
        side.turns_started = 4;
        side.mana.current = current;
        side.mana.max = max;
        side.mana.perm_mod = 0;
    }

    #[test]
    fn r3569008_max_mana_drops_for_the_rest_of_the_game() {
        let mut s = board(
            json!({ "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER] }),
            json!({ "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER] }),
        );
        at(&mut s, 4, 4);
        lose(&mut s, 1);
        assert_eq!(mana_of(&s), (3, 3), "max and current both go down");
        assert_eq!(s.state().players[P1].mana.perm_mod, -1);
        // The next refresh is to the lowered max, not back to 4.
        s.end_turn().end_turn();
        assert_eq!(s.state().active, P1);
        assert_eq!(mana_of(&s), (3, 3));
    }

    #[test]
    fn r3569008_an_empty_crystal_goes_first() {
        let mut s = board(json!({}), json!({}));
        at(&mut s, 2, 4);
        lose(&mut s, 1);
        assert_eq!(mana_of(&s), (2, 3), "the spent crystal was the one lost");
        lose(&mut s, 2);
        assert_eq!(mana_of(&s), (1, 1), "then the full ones, down to what is left");
    }

    #[test]
    fn r3569008_max_mana_floors_at_zero_and_a_nothing_changes_nothing() {
        let mut s = board(json!({}), json!({}));
        at(&mut s, 4, 4);
        lose(&mut s, 0);
        assert_eq!(mana_of(&s), (4, 4));
        lose(&mut s, -3);
        assert_eq!(mana_of(&s), (4, 4));
        lose(&mut s, 10);
        assert_eq!(mana_of(&s), (0, 0));
        // With no empty crystal to lose, a full one goes: temporary mana above max keeps its excess.
        at(&mut s, 7, 4);
        lose(&mut s, 1);
        assert_eq!(mana_of(&s), (6, 3));
    }
}
