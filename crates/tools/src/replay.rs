//! `cargo jackioh replay` (SPEC §9.2, §9.3; SURFACE §12): folds a recorded game from scratch and prints
//! its state hash. Replaces the TS fold that `e2e/support/tasks/replay-runner.ts` ran under tsx.
//!
//! Reads `{seed, decks, log, handicaps?, dealt?, lastBoards?, glitchBoards?, catalogVersion?}` as JSON on
//! stdin and prints one JSON line, `{"hash": hashState(folded state), "errors": [{nonce, error}, …]}`. The
//! setup fields are the fold's input exactly as the live `createGame` had them (R180 handicaps, R433
//! dealt seats, R417 last boards, R678 Glitch boards); any other key (a browser's `state`, say) is
//! ignored. A `catalogVersion` folds the game under that shipped catalog (R767000, `catalog_for` inside
//! `with_shipped`, as `replay_open` does); one this build neither is nor carries exits 1 naming
//! `earlier_patch` and prints no hash, and without it the game folds under this build's catalog. A
//! co-op game, `{options, log}` (`DuosReplayInput`, pinned by `options.catalogVersion`), is folded by
//! `duos_fold` and answers `{"hash": duos_hash, "refusedAt": index or null}`; its fold stops at the
//! first refused action.
//! The card scripts are registered first: folding without them would diverge from the browser
//! instead of matching it, since every Cry in the log would fizzle differently (SPEC §10.9). A game
//! with custom cards (`customCards`, R915006) folds with the card kit's host lent to the engine, as
//! the browser's Luau module folds it (R915009), so a practice game with a custom card replays here
//! to the browser's hash.
//!
//! A log may hold rejects, so a refused action is listed in `errors`, not fatal. The setup is not:
//! a deck its seat's handicap does not allow fails loudly instead of replaying a different game
//! (R180), and input that is not such an object exits 1.

use std::io::Read as _;
use std::rc::Rc;

use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};

use jackioh_engine::custom::with_custom_scripts;
use jackioh_engine::replay::{FoldArgs, fold, hash_state};
use jackioh_engine::{DuosReplayInput, ShippedCatalog, catalog_for, duos_fold, with_shipped};
use jackioh_luau::custom::KitHost;

/// `cargo jackioh replay`: no options; the game comes on stdin.
#[derive(clap::Args, Debug)]
pub struct Args {}

/// R767000: the shipped catalog `game` folds under, read off its `catalogVersion`: `None` (this
/// build's) when it names none or this build's own version, the carried one when this build carries
/// it, and an `earlier_patch` error, before anything is folded, for any other version.
pub(crate) fn pinned_catalog(game: &Value) -> Result<Option<&'static ShippedCatalog>> {
    let version = match game.get("catalogVersion") {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::String(version)) => version,
        Some(other) => bail!("catalogVersion {other} is not a string"),
    };
    let build = jackioh_cards::catalog_version();
    catalog_for(version, build).map_err(|reason| {
        anyhow!(
            "{reason}: the game was played on catalog {version}, which this build ({build}) neither is \
             nor carries (R767000); fold it with a build of that version"
        )
    })
}

/// A co-op game (`DuosReplayInput`'s `{options, log}`), pinned by `options.catalogVersion` as a 1v1
/// game is by `catalogVersion`, folded by `duos_fold`: `{"hash": duos_hash, "refusedAt": the index in
/// `log` of the action the fold refused and stopped at, or null}`.
fn duos_json(input: Value) -> Result<Value> {
    let shipped = pinned_catalog(&input["options"])?;
    let game: DuosReplayInput =
        serde_json::from_value(input).context("the input is not a co-op game {options, log}")?;
    let folded = with_shipped(shipped, || duos_fold(&game));
    Ok(json!({ "hash": folded.hash, "refusedAt": folded.refused_at }))
}

/// Folds one game's JSON: a co-op `{options, log}` through `duos_json`, else `FoldArgs`'s shape under
/// its pinned catalog; answers `{"hash", "errors"}`.
fn replay_json(input: Value) -> Result<Value> {
    if input.get("options").is_some() {
        return duos_json(input);
    }
    let shipped = pinned_catalog(&input)?;
    let args: FoldArgs = serde_json::from_value(input).context("the input is not {seed, decks, log, …}")?;
    let folded = with_shipped(shipped, || with_custom_scripts(Rc::new(KitHost), || fold(&args)));
    Ok(json!({
        "hash": hash_state(&folded.state),
        "errors": serde_json::to_value(&folded.errors)?,
    }))
}

/// `cargo jackioh replay`.
pub fn run(_args: Args) -> Result<()> {
    let mut text = String::new();
    std::io::stdin()
        .read_to_string(&mut text)
        .context("reading stdin")?;
    let input: Value = serde_json::from_str(&text).context("stdin is not JSON")?;
    jackioh_cards::register_all();
    println!("{}", serde_json::to_string(&replay_json(input)?)?);
    Ok(())
}

/// The version `game_on_a_shipped_catalog` carries for its test alone, and how it finds its game.
#[cfg(test)]
pub(crate) const SHIPPED_TEST_VERSION: &str = "v-replay-test";
#[cfg(test)]
pub(crate) const SHIPPED_SEEDS_TRIED: u32 = 8;
#[cfg(test)]
const SHIPPED_GAME_ACTIONS: usize = 120;

/// R767000: a game played under a catalog the calling test's thread carries as `SHIPPED_TEST_VERSION`
/// (the build's with every Unit one Attack higher on both faces): `fuzz`'s decks for the first of
/// `SHIPPED_SEEDS_TRIED` seeds whose log folds to another hash under the build's own catalog, both
/// seats random. Answers its JSON, `catalogVersion` included, and its live hash. The caller holds
/// `preview_every_set()` (the fuzz decks may hold a set that has not shipped, R1420) and runs
/// `clear_overrides()`.
#[cfg(test)]
pub(crate) fn game_on_a_shipped_catalog() -> (Value, String) {
    use indexmap::IndexMap;
    use jackioh_ai::{MatchConfig, MatchHooks, SeatController, play_match};
    use jackioh_engine::PerPlayer;
    use jackioh_engine::testkit::register_shipped;
    jackioh_cards::register_all();
    let mut stronger = jackioh_engine::registered_catalog().clone();
    for def in stronger.values_mut() {
        def.base.attack = def.base.attack.map(|attack| attack + 1);
        def.radiant.attack = def.radiant.attack.map(|attack| attack + 1);
    }
    register_shipped(vec![ShippedCatalog::new(
        SHIPPED_TEST_VERSION,
        stronger,
        IndexMap::new(),
    )]);
    let shipped = jackioh_engine::shipped_catalog(SHIPPED_TEST_VERSION);
    for n in 1..=SHIPPED_SEEDS_TRIED {
        let seed = format!("replay-shipped-{n}");
        let decks = crate::fuzz::decks_for_seed(n);
        let config = MatchConfig {
            seed: seed.clone(),
            decks: decks.clone(),
            handicaps: None,
            controllers: PerPlayer::new(SeatController::Random, SeatController::Random),
            max_actions: Some(SHIPPED_GAME_ACTIONS),
        };
        let record = with_shipped(shipped, || play_match(&config, &mut MatchHooks::default()));
        let mut game = json!({ "seed": seed, "decks": [decks.0, decks.1], "log": record.log });
        let args: FoldArgs = serde_json::from_value(game.clone()).unwrap();
        if hash_state(&fold(&args).state) != record.hash {
            game["catalogVersion"] = json!(SHIPPED_TEST_VERSION);
            return (game, record.hash);
        }
    }
    panic!("no seed's game folds to another hash under the build's catalog");
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::subsystems::AI_SKIPPED_ACTIONS;
    use jackioh_engine::testkit::{clear_overrides, preview_every_set};
    use jackioh_engine::{
        DuosAction, DuosActionBody, DuosCreateOptions, Rng, duos_begin, duos_create, duos_hash,
        duos_legal_actions, duos_reduce, duos_seats_to_act,
    };

    /// The hotseat game e2e spec 01 recorded (SURFACE §13), committed beside the golden traces.
    const HOTSEAT: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../engine/tests/golden/01-hotseat-full-game.json"
    );
    /// Its pinned hash (`packages/cards/test/hotseat-replay.test.ts`).
    const HOTSEAT_HASH: &str = "5c3d578c";

    #[test]
    fn folds_the_hotseat_game_to_its_pinned_hash_with_no_errors() {
        jackioh_cards::register_all();
        let input: Value = serde_json::from_str(&std::fs::read_to_string(HOTSEAT).unwrap()).unwrap();
        let answer = replay_json(input).unwrap();
        assert_eq!(answer["hash"], HOTSEAT_HASH);
        assert_eq!(answer["errors"], json!([]));
    }

    #[test]
    fn a_refused_action_is_listed_with_its_nonce_and_changes_nothing() {
        jackioh_cards::register_all();
        let mut input: Value = serde_json::from_str(&std::fs::read_to_string(HOTSEAT).unwrap()).unwrap();
        // An attack by a card that does not exist, after the game's last action: refused, listed, and
        // the fold goes on to the same state.
        let refused = json!({
            "type": "attack", "attackerId": "no-such-card", "targetId": "hero-p2", "playerId": "p1", "nonce": "refused-1"
        });
        input["log"].as_array_mut().unwrap().push(refused);
        let answer = replay_json(input).unwrap();
        assert_eq!(answer["hash"], HOTSEAT_HASH);
        let errors = answer["errors"].as_array().unwrap();
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0]["nonce"], "refused-1");
        assert!(
            errors[0]["error"].as_str().is_some_and(|error| !error.is_empty()),
            "{errors:?}"
        );
    }

    /// A 0-cost Spell that deals `{dmg}` (3) to the enemy hero, in the card kit.
    fn burner() -> Value {
        json!({
            "id": "custom-burner",
            "version": 1,
            "frame": {
                "name": "Burner", "type": "Spell", "cost": 0, "rarity": "Common",
                "text": "Deal {dmg} damage to the enemy hero.",
                "params": [{ "key": "dmg", "base": 3, "radiant": 6 }]
            },
            "source": "return Card {\n  cry = function(c, n)\n    c:damage(\"the enemy hero\", n.dmg)\n  end,\n}\n"
        })
    }

    /// practice/decks.ts' Human Vanguard: twenty distinct Core cards.
    const VANGUARD: [&str; 20] = [
        "core-004", "core-005", "core-008", "core-011", "core-015", "core-081", "core-001", "core-016",
        "core-020", "core-045", "core-061", "core-069", "core-077", "core-030", "core-013", "core-019",
        "core-053", "core-025", "core-054", "core-066",
    ];

    fn fold_of(input: &Value, host: bool) -> jackioh_engine::replay::ReplayResult {
        let args: FoldArgs = serde_json::from_value(input.clone()).unwrap();
        if host {
            with_custom_scripts(Rc::new(KitHost), || fold(&args))
        } else {
            fold(&args)
        }
    }

    fn view_of(state: &jackioh_engine::GameState, seat: jackioh_engine::PlayerId) -> Value {
        serde_json::to_value(jackioh_engine::view_for::view_for(state, seat)).unwrap()
    }

    /// R915006: a practice game whose deck holds a custom card replays here to the browser's state: the
    /// card's Cry runs through the card kit's host, and without the host it would do nothing.
    #[test]
    fn r915006_a_game_with_a_custom_card_folds_through_the_card_kit() {
        jackioh_cards::register_all();
        let mut human: Vec<String> = vec!["custom-burner".to_string()];
        human.extend(VANGUARD[..19].iter().map(|id| (*id).to_string()));
        let ai: Vec<String> = VANGUARD.iter().map(|id| (*id).to_string()).collect();
        // A seed whose opening hand holds the card: the deal is the seed's, so the search is fixed.
        let mut input = Value::Null;
        for n in 0..60 {
            let candidate = json!({
                "seed": format!("r915006-tools-{n}"), "decks": [human, ai], "log": [], "customCards": [burner()]
            });
            let state = fold_of(&candidate, true).state;
            let view = view_of(&state, jackioh_engine::PlayerId::P1);
            let holds = view["you"]["hand"]
                .as_array()
                .is_some_and(|hand| hand.iter().any(|card| card["defId"] == "custom-burner"));
            if holds {
                input = candidate;
                break;
            }
        }
        assert!(
            !input.is_null(),
            "no seed in 60 deals the custom card into the opening hand"
        );

        // Keep both hands, pass until the human's main phase, then play the card.
        let mut played_health = None;
        for step in 0..40 {
            let state = fold_of(&input, true).state;
            let Some(seat) = jackioh_engine::seat_to_act(&state) else {
                break;
            };
            let view = view_of(&state, seat);
            let pending = &view["pending"];
            let mut action = if pending["kind"] == "mulligan" {
                let keep: Vec<Value> = pending["options"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|option| option["key"].clone())
                    .collect();
                json!({ "type": "mulligan", "keep": keep })
            } else if seat == jackioh_engine::PlayerId::P1 && view["phase"] == "main" && pending.is_null() {
                let burner = view["you"]["hand"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|card| card["defId"] == "custom-burner")
                    .unwrap();
                played_health = Some(view["opponent"]["hero"]["health"].as_i64().unwrap());
                json!({ "type": "play", "instanceId": burner["instanceId"].clone() })
            } else {
                json!({ "type": "endTurn" })
            };
            action["playerId"] = serde_json::to_value(seat).unwrap();
            action["nonce"] = json!(format!("n{step}"));
            let legal = jackioh_engine::legal_actions(&state, seat);
            let body: jackioh_engine::ActionBody = serde_json::from_value(action.clone()).unwrap();
            assert!(
                action["type"] == "mulligan" || legal.contains(&body),
                "{action} is not legal"
            );
            input["log"].as_array_mut().unwrap().push(action);
            if played_health.is_some() {
                break;
            }
        }
        let before = played_health.expect("the human reaches a main phase with the card in hand");

        let answer = replay_json(input.clone()).unwrap();
        assert_eq!(answer["errors"], json!([]));
        let folded = fold_of(&input, true);
        assert_eq!(answer["hash"], hash_state(&folded.state));
        let after = view_of(&folded.state, jackioh_engine::PlayerId::P1)["opponent"]["hero"]["health"]
            .as_i64()
            .unwrap();
        assert_eq!(after, before - 3, "the Cry dealt its {{dmg}}");
        // Without the host the card has no script: the same log folds to another game.
        let bare = fold_of(&input, false);
        assert_ne!(hash_state(&bare.state), answer["hash"].as_str().unwrap());
    }

    #[test]
    fn input_that_is_no_game_is_refused() {
        assert!(replay_json(json!({ "seed": "x" })).is_err());
        assert!(replay_json(json!([1, 2, 3])).is_err());
    }

    /// Actions a co-op test game plays at most.
    const COOP_GAME_ACTIONS: usize = 200;

    /// A seeded co-op game, all under `shipped`: `duos_fuzz`'s decks for `seed`, then up to
    /// `COOP_GAME_ACTIONS` actions, the seat and the action drawn from the seed's own stream among what
    /// may be taken (never a concede or an emote, as `duos_fuzz`'s policy). Its `{options, log}` and live hash.
    fn coop_game(seed: u32, shipped: Option<&'static ShippedCatalog>) -> (Value, String) {
        with_shipped(shipped, || {
            let options = DuosCreateOptions {
                seed: format!("replay-coop-{seed}"),
                decks: crate::duos_fuzz::duos_decks_for_seed(seed),
                ..DuosCreateOptions::default()
            };
            let mut state = duos_begin(&duos_create(&options)).state;
            let mut rng = Rng::new(&format!("replay-coop-policy-{seed}"), 0);
            let mut log: Vec<DuosAction> = Vec::new();
            while log.len() < COOP_GAME_ACTIONS && state.result.is_none() {
                let seats = duos_seats_to_act(&state);
                let Some(&seat) = rng.pick(&seats) else { break };
                let actions: Vec<DuosActionBody> = duos_legal_actions(&state, seat)
                    .into_iter()
                    .filter(|action| match action {
                        DuosActionBody::Concede => false,
                        DuosActionBody::Duel { body } => !AI_SKIPPED_ACTIONS.contains(&body.action_type()),
                        DuosActionBody::Pass { .. } => true,
                    })
                    .collect();
                let Some(body) = rng.pick(&actions).cloned() else {
                    break;
                };
                let action = DuosAction {
                    seat,
                    nonce: format!("coop-{}", log.len()),
                    body,
                };
                let result = duos_reduce(&state, &action);
                assert!(result.error.is_none(), "{:?}", result.error);
                state = result.state;
                log.push(action);
            }
            (
                serde_json::to_value(DuosReplayInput { options, log }).unwrap(),
                duos_hash(&state),
            )
        })
    }

    fn hotseat_game() -> Value {
        jackioh_cards::register_all();
        serde_json::from_str(&std::fs::read_to_string(HOTSEAT).unwrap()).unwrap()
    }

    /// R767000: a game played on a catalog the build carries folds under it to the hash it had live, a
    /// 1v1 game by its `catalogVersion` and a co-op game by its `options.catalogVersion`; a fold under
    /// the build's own catalog would reach another.
    #[test]
    fn r767000_a_game_on_a_carried_catalog_folds_to_its_live_hash_1v1_and_co_op() {
        let _preview = preview_every_set();
        let (game, live) = game_on_a_shipped_catalog();
        let answer = replay_json(game).unwrap();
        assert_eq!(answer["hash"], live);
        assert_eq!(answer["errors"], json!([]));

        let shipped = jackioh_engine::shipped_catalog(SHIPPED_TEST_VERSION);
        let (mut game, live) = (1..=SHIPPED_SEEDS_TRIED)
            .map(|n| coop_game(n, shipped))
            .find(|(game, live)| {
                let bare = duos_fold(&serde_json::from_value(game.clone()).unwrap());
                bare.hash != *live || bare.refused_at.is_some()
            })
            .expect("no seed's co-op game folds differently under the build's catalog");
        game["options"]["catalogVersion"] = json!(SHIPPED_TEST_VERSION);
        let answer = replay_json(game).unwrap();
        assert_eq!(answer["hash"], live);
        assert_eq!(answer["refusedAt"], Value::Null);
        clear_overrides();
    }

    /// R767000: a version the build neither is nor carries is refused as `earlier_patch` before
    /// anything is folded, so no hash is printed; the build's own version folds as no version does.
    #[test]
    fn r767000_a_catalog_the_build_neither_is_nor_carries_is_refused_as_earlier_patch() {
        let _preview = preview_every_set();
        let mut game = hotseat_game();
        game["catalogVersion"] = json!("v-gone");
        let error = replay_json(game).unwrap_err();
        assert!(format!("{error:#}").contains("earlier_patch"), "{error:#}");

        let (mut game, _) = coop_game(1, None);
        game["options"]["catalogVersion"] = json!("v-gone");
        let error = replay_json(game).unwrap_err();
        assert!(format!("{error:#}").contains("earlier_patch"), "{error:#}");

        let mut game = hotseat_game();
        game["catalogVersion"] = json!(jackioh_cards::catalog_version());
        let answer = replay_json(game).unwrap();
        assert_eq!(answer["hash"], HOTSEAT_HASH);
        assert_eq!(answer["errors"], json!([]));

        let mut game = hotseat_game();
        game["catalogVersion"] = json!(7);
        assert!(replay_json(game).is_err());
    }

    /// A co-op game folds to its live `duos_hash` with no `refusedAt`; an action the engine refuses
    /// after it is named by its index in the log, and stops the fold there.
    #[test]
    fn a_co_op_game_folds_to_its_live_duos_hash_and_names_the_action_it_refused() {
        let _preview = preview_every_set();
        let (mut game, live) = coop_game(1, None);
        let answer = replay_json(game.clone()).unwrap();
        assert_eq!(answer["hash"], live);
        assert_eq!(answer["refusedAt"], Value::Null);

        let log = game["log"].as_array_mut().unwrap();
        let at = log.len();
        log.push(json!({
            "seat": "a1", "nonce": "refused-1", "body": { "kind": "pass", "instanceId": "no-such-card" }
        }));
        let answer = replay_json(game).unwrap();
        assert_eq!(answer["refusedAt"], json!(at));
        assert_eq!(answer["hash"], live);
    }
}
