//! A lesson's random pools over the real cards (SPEC §5.1, §9.10; R2201001): random-policy games whose
//! decks hold no Legendary or Mythic card, one seat under `TUTORIAL_HUMAN` with `poolHold`. No
//! Legendary or Mythic card ever appears anywhere in such a game, a prompt's options included: every
//! card one of them shows was generated, and a held game generates none. The same games unheld do
//! show some, so the sweep reaches the pools it guards.

use jackioh_cards::{CATALOG, register_all};
use jackioh_engine::catalog::{HELD_RARITIES, deckable, made_by_a_rule, self_def_ids};
use jackioh_engine::testkit::*;

const GAMES: i32 = 15;

/// The cards whose text names the Legendary or Mythic card it makes, so no pool is drawn: a named
/// card reaches its hand or deck under the hold (R2201001), as a named card does under R1420's gate.
const NAMES_A_HELD_CARD: &[&str] = &[
    "stinkyscape-008", // Conjure Rosemary Triscuit: "Shuffle a Wardrum into your deck."
    "stinkyscape-095", // Vanguard Coalition: "Add a Scornboi, Ocean Tsunami, Tylannt, … to your hand."
    "stinkyscape-159", // ETHICS: "Add an iKwzki, Shockwave447, xxFreshJack, … Tylannt to your hand."
    "stinkyscape-160", // Golden Accord: "Add a Paragon Mk1, Paragon Mk2, Paragon-X and Falcon …"
    "stinkyscape-162", // Star-jumper Core: "Add a Badger, Honey Badger, Ozark, Orca and Galaxy …"
    // YileGPT Prime, a token: "Cry: … summon Claude's Datacenters". A random game reaches it as a
    // Marketplace buy at (0) since #3047 (R3047001).
    "meditative-049-1",
    // The Stinkyscape+ release (#3573): the two cards of the set that name a held card, as a Faction's
    // text does, so no pool is drawn.
    "stinkyscapeplus-120", // Crawler: "add a Toxopid to your hand" (S+ #121, Legendary).
    "stinkyscapeplus-136", // Crawler Assembly Line: "Summon a Crawler", whose Death adds a Toxopid.
    "stinkyscapeplus-137", // The Perfect Pair: "The pairs are Scepter and Reign, … Quad and Oct, …" (R3565004).
];
/// A held card the text of another card names, so it reaches a game wherever that other card stands, not
/// only as it is played: Toxopid (S+ #121) comes with each Crawler (#120), which any Mindustrial pool
/// can generate and which adds it by name as it dies, however it arrived.
const NAMED_BY_A_CARD_IN_PLAY: &[(&str, &str)] = &[("stinkyscapeplus-121", "stinkyscapeplus-120")];

/// Whether `def_id` is one of `ids`, or a fused card (R179) with one of them among its ingredients:
/// a fused card runs every ingredient's scripts (R77), so it names whatever they name.
fn stands_for_any(state: Option<&GameState>, def_id: &str, ids: &[&str]) -> bool {
    self_def_ids(state, def_id)
        .iter()
        .any(|id| ids.contains(&id.as_str()))
}

/// Whether `state` holds a card whose text names the held card `id` (`NAMED_BY_A_CARD_IN_PLAY`).
fn named_by_a_card_in_play(state: &GameState, id: &str) -> bool {
    let text = serde_json::to_string(state).expect("the state serializes");
    NAMED_BY_A_CARD_IN_PLAY.iter().any(|(named, namer)| {
        *named == id
            && text
                .split("\"defId\":\"")
                .skip(1)
                .filter_map(|piece| piece.split('"').next())
                .any(|def_id| stands_for_any(Some(state), def_id, &[*namer]))
    })
}

const MAX_ACTIONS: usize = 3000;

fn unheld_pool() -> Vec<String> {
    let mut ids: Vec<String> = CATALOG
        .iter()
        .filter(|(id, def)| {
            deckable(def)
                && !def.tags.contains(&Tag::Token)
                && !HELD_RARITIES.contains(&def.rarity)
                && !NAMES_A_HELD_CARD.contains(&id.as_str())
        })
        .map(|(id, _)| id.clone())
        .collect();
    ids.sort();
    ids
}

fn held_named(state: &GameState) -> Vec<String> {
    let text = serde_json::to_string(state).expect("the state serializes");
    let mut named: Vec<String> = Vec::new();
    for marker in ["\"defId\":\"", "\"mode:"] {
        for piece in text.split(marker).skip(1) {
            let id = piece.split('"').next().unwrap_or_default();
            // R1857001: a card only a rule makes (the Dormant Starship) is drawn from no pool.
            let held = CATALOG
                .get(id)
                .is_some_and(|def| HELD_RARITIES.contains(&def.rarity) && !made_by_a_rule(def));
            if held && !named.iter().any(|n| n == id) {
                named.push(id.to_string());
            }
        }
    }
    named
}

fn sweep(n: i32, hold: bool) -> Vec<(String, usize)> {
    let pool = unheld_pool();
    let human = TUTORIAL_HUMAN.deck_size as usize;
    let shuffled = create_rng(&format!("r2201001-decks-{n}"), 0).shuffle(&pool);
    let decks = (
        shuffled[..human].to_vec(),
        shuffled[human..human + DECK_SIZE as usize].to_vec(),
    );
    let seed = format!("r2201001-sweep-{n}");
    let mut state = begin_game(&create_game(&CreateGameOptions {
        seed: seed.clone(),
        decks,
        handicaps: Some(PerPlayerOpt {
            p1: Some(Handicap {
                pool_hold: hold.then_some(true),
                ..TUTORIAL_HUMAN
            }),
            p2: None,
        }),
        ..CreateGameOptions::default()
    }))
    .state;
    let mut policy = create_rng(&format!("r2201001-policy-{n}"), 0);
    let mut seen: Vec<(String, usize)> = Vec::new();
    let mut named_made: Vec<String> = Vec::new();
    let mut steps = 0;
    while state.result.is_none() && steps < MAX_ACTIONS {
        let player = seat_to_act(&state).unwrap_or_else(|| panic!("{seed}: no seat to act"));
        let Some(chosen) = subsystems::choose_action(&state, player, &mut policy) else {
            panic!("{seed}: no action for {}", player.as_str());
        };
        let names_one = match &chosen {
            ActionBody::Play { instance_id, .. } => find_instance(&state, instance_id)
                .is_some_and(|card| stands_for_any(Some(&state), &card.def_id, NAMES_A_HELD_CARD)),
            _ => false,
        };
        let result = reduce(&state, &Action::new(chosen, player, format!("s{steps}")));
        if let Some(error) = &result.error {
            panic!("{seed}: refused: {error}");
        }
        let names_one = names_one
            || result.events.iter().any(|event| {
                matches!(event, GameEvent::CardPlayed { def_id, .. }
                    if stands_for_any(Some(&result.state), def_id, NAMES_A_HELD_CARD))
            });
        state = result.state;
        steps += 1;
        for id in held_named(&state) {
            if named_by_a_card_in_play(&state, &id) {
                continue;
            }
            if names_one && !seen.iter().any(|(named, _)| *named == id) {
                named_made.push(id.clone());
            }
            if !named_made.contains(&id) && !seen.iter().any(|(named, _)| *named == id) {
                seen.push((id, steps));
            }
        }
    }
    seen
}

mod r2201001_a_held_lesson_shows_no_legendary_or_mythic_card {
    use super::*;

    #[test]
    fn r2201001_random_games_under_the_hold_never_show_one_and_the_same_games_unheld_do() {
        register_all();
        let mut unheld_shown = 0;
        for n in 1..=GAMES {
            let held = sweep(n, true);
            assert!(held.is_empty(), "game {n} under the hold showed {held:?}");
            if unheld_shown == 0 && !sweep(n, false).is_empty() {
                unheld_shown += 1;
            }
        }
        assert!(
            unheld_shown > 0,
            "no unheld game showed a Legendary or Mythic card: the sweep reaches no pool"
        );
    }

    #[test]
    fn r2201001_a_fused_card_names_the_held_cards_its_ingredients_name() {
        // The Perfect Pair fused (R77, R179) still adds Quad and Oct by name: the sweep's game 4 at
        // v0.6.7k (#4272) played this very card.
        let fused = "t-13:stinkyscapeplus-137+stinkyscapeplus-135";
        assert!(stands_for_any(None, fused, NAMES_A_HELD_CARD));
        assert!(stands_for_any(
            None,
            "t-2:(t-1:stinkyscapeplus-135+stinkyscapeplus-137)+core-001",
            NAMES_A_HELD_CARD
        ));
        assert!(stands_for_any(None, "stinkyscapeplus-137", NAMES_A_HELD_CARD));
        assert!(!stands_for_any(None, "stinkyscapeplus-135", NAMES_A_HELD_CARD));
        assert!(!stands_for_any(
            None,
            "t-1:stinkyscapeplus-135+core-001",
            NAMES_A_HELD_CARD
        ));
        // A fused Crawler adds its Toxopid as the bare Crawler does.
        assert!(stands_for_any(
            None,
            "t-3:stinkyscapeplus-120*+core-001",
            &["stinkyscapeplus-120"]
        ));
    }
}
