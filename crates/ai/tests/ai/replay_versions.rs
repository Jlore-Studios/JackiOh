//! R767000, issue #767: a practice-shaped game (the AI on the medium handicap against a random
//! player, on the real cards) played on catalog version N replays step for step after a patch to N+1
//! that changes a card it played. Folded under N's catalog, every step's view of both seats is the one
//! the game showed; a version the build does not carry is `earlier_patch`, and a wrong final hash
//! `rules_changed`.

use jackioh_ai::{MatchConfig, MatchHooks, SeatController, play_match};
use jackioh_engine::config::REPLAY_PAGE_STEPS;
use jackioh_engine::testkit::{
    AI_DIFFICULTY, CardDefs, DECK_SIZE, IndexMap, PLAYER_IDS, PerPlayer, PerPlayerOpt, PlayerView,
    ReplayInput, ReplayOpen, ReplayRecord, ReplayRefusal, ShippedCatalog, clear_overrides, fold, hash_state,
    register_catalog_as, register_shipped, registered_catalog, replay_open, replay_page, view_for,
};

use super::support::{random_decks_sized, register_cards};

/// How many seeds the test plays looking for games a patch changes.
const SEEDS_TRIED: u32 = 8;
/// The version the games are played on, and the build's after the patch.
const PLAYED_ON: &str = "v-n";
const BUILD: &str = "v-n+1";

/// `catalog` with `unit` one attack higher on both faces.
fn patched(catalog: &CardDefs, unit: &str) -> CardDefs {
    let mut patched = catalog.clone();
    if let Some(def) = patched.get_mut(unit) {
        def.base.attack = def.base.attack.map(|attack| attack + 1);
        def.radiant.attack = def.radiant.attack.map(|attack| attack + 1);
    }
    patched
}

#[test]
fn r767000_a_practice_game_on_version_n_replays_step_for_step_after_a_patch_to_n_plus_1() {
    register_cards();
    let on_n = registered_catalog().clone();
    let medium = AI_DIFFICULTY.medium;
    let mut proved = 0;
    for n in 1..=SEEDS_TRIED {
        let seed = format!("replay-versions-{n}");
        let decks = random_decks_sized(&seed, (medium.deck_size, DECK_SIZE));
        let handicaps = PerPlayerOpt {
            p1: Some(medium),
            p2: None,
        };
        let record = play_match(
            &MatchConfig {
                seed: seed.clone(),
                decks: decks.clone(),
                handicaps: Some(handicaps.clone()),
                controllers: PerPlayer::new(SeatController::Greedy, SeatController::Random),
                max_actions: Some(120),
            },
            &mut MatchHooks::default(),
        );
        let input = |log| ReplayInput {
            seed: seed.clone(),
            decks: decks.clone(),
            log,
            handicaps: Some(handicaps.clone()),
            ..ReplayInput::default()
        };
        // Both seats' views of every step, as the game on N showed them.
        let shown: Vec<[PlayerView; 2]> = (0..=record.log.len())
            .map(|k| {
                let folded = fold(&input(record.log[..k].to_vec()));
                assert!(folded.errors.is_empty(), "{seed} step {k}");
                PLAYER_IDS.map(|seat| view_for(&folded.state, seat))
            })
            .collect();
        let Some(unit) = record
            .played
            .p1
            .iter()
            .chain(record.played.p2.iter())
            .find(|id| on_n.get(*id).is_some_and(|def| def.base.attack.is_some()))
        else {
            continue;
        };

        register_shipped(vec![ShippedCatalog::new(
            PLAYED_ON,
            on_n.clone(),
            IndexMap::new(),
        )]);
        register_catalog_as(patched(&on_n, unit), BUILD);
        let game = input(record.log.clone());
        if hash_state(&fold(&game).state) == record.hash {
            // The patch changed nothing this game did: it proves nothing.
            clear_overrides();
            continue;
        }
        let on = |version: &str, hash: &str| ReplayRecord {
            catalog_version: version.to_string(),
            final_hash: hash.to_string(),
        };
        let ReplayOpen::Ready { steps, checkpoints } =
            replay_open(&game, &on(PLAYED_ON, &record.hash), BUILD)
        else {
            panic!("{seed}: a game on a carried version was refused");
        };
        assert_eq!(steps, shown.len(), "{seed}");
        for (at, seat) in PLAYER_IDS.into_iter().enumerate() {
            let mut from = 0;
            while from < steps {
                let page = replay_page(&checkpoints, seat, from, REPLAY_PAGE_STEPS);
                assert!(!page.steps.is_empty(), "{seed} {seat} {from}");
                for step in &page.steps {
                    assert_eq!(
                        step.view, shown[step.step][at],
                        "{seed} {seat} step {}",
                        step.step
                    );
                }
                from += page.steps.len();
            }
        }
        let refused = |reason| ReplayOpen::Refused { reason };
        assert_eq!(
            replay_open(&game, &on("v-gone", &record.hash), BUILD),
            refused(ReplayRefusal::EarlierPatch)
        );
        assert_eq!(
            replay_open(&game, &on(PLAYED_ON, "00000000"), BUILD),
            refused(ReplayRefusal::RulesChanged)
        );
        clear_overrides();
        proved += 1;
    }
    assert!(proved > 0, "no seed played a unit whose patch changes its game");
}
