//! Port of `packages/engine/test/mana-before-play.test.ts`.
//!
//! `EffectContext.manaBeforePlay` (Classic #22 Mid Runner: "If you had 4 or more mana when you played
//! this"): the player's current mana as the play began — at §10.5 step 1, before step 2 pays — or as a
//! cast began, handed to the played card's own Cry and every continuation of it, across a pause and a
//! JSON round trip.

use jackioh_engine::effects::{choose_mode, damage};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::in_hand;
use crate::rules::fixtures::play_pipeline_b::{only, pb_act, pb_playing, round_trip};

fn unit(name: &str, index: i32) -> CardDef {
    json_as(json!({
        "id": format!("pbm-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (mana before play)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 2,
        "base": { "attack": 2, "health": 2, "keywords": [], "text": name },
        "radiant": { "attack": 4, "health": 4, "keywords": [], "text": name },
    }))
}

/// Its Cry deals 4 when its player had 4 or more mana as they played it, else 1.
fn runner() -> CardDef {
    unit("runner", 4621)
}

/// The same, read in the step a prompt re-enters.
fn asking_runner() -> CardDef {
    unit("asking-runner", 4622)
}

fn hit(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let amount = if ctx.mana_before_play.unwrap_or(0) >= 4 { 4 } else { 1 };
    vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))]
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    scripts.insert(
        runner().id,
        CardScripts {
            base: Script { cry: Some(hook(hit)), ..Script::default() },
            radiant: Script { cry: Some(hook(hit)), ..Script::default() },
        },
    );
    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    resume.insert("go", hook(hit));
    scripts.insert(
        asking_runner().id,
        CardScripts {
            base: Script {
                cry: Some(hook(|_ctx| vec![choose_mode(json_as(json!({ "options": ["go"], "step": "go" })))])),
                resume,
                ..Script::default()
            },
            radiant: Script::default(),
        },
    );
    scripts
}

fn playing(seed: &str, mana: i32) -> GameState {
    let mut state = pb_playing(seed);
    let mut defs = registered_catalog().clone();
    defs.insert(runner().id, runner());
    defs.insert(asking_runner().id, asking_runner());
    register_catalog(defs);
    let mut all = registered_scripts().clone();
    all.extend(scripts());
    register_scripts(all);
    state.players.p1.mana.current = mana;
    state
}

/// TS `sinkFor(state)`: a sink's events and rng (from the state's cursor), lent with the state to one
/// engine call at a time.
struct Bench {
    events: Vec<GameEvent>,
    rng: Rng,
}

impl Bench {
    fn new(state: &GameState) -> Bench {
        Bench { events: Vec::new(), rng: Rng::new(&state.seed, state.rng_cursor) }
    }

    fn sink<'a>(&'a mut self, state: &'a mut GameState) -> EngineSink<'a> {
        EngineSink::new(state, &mut self.events, &mut self.rng)
    }
}

/// TS `{ ...body, playerId }`: an action body with its sender, as the JSON the fixtures take.
fn with_player(body: &ActionBody, player: PlayerId) -> Value {
    let mut value = serde_json::to_value(body).expect("an action body serialises");
    value["playerId"] = json!(player);
    value
}

mod the_players_mana_as_the_play_began_classic_22 {
    use super::*;

    #[test]
    fn reads_the_mana_before_step_2_paid_not_after() {
        let mut rich = playing("mana-before-4", 4);
        let card = only(&in_hand(&mut rich, &runner().id, PlayerId::P1, 1));
        let after = pb_act(
            &rich,
            json_as(json!({ "type": "play", "instanceId": card.id, "zone": { "row": "units", "lane": 1 }, "playerId": "p1" })),
        );
        assert_eq!(after.players.p1.mana.current, 2);
        assert_eq!(after.players.p2.hero.health, rich.players.p2.hero.health - 4);

        let mut poor = playing("mana-before-3", 3);
        let other = only(&in_hand(&mut poor, &runner().id, PlayerId::P1, 1));
        let then = pb_act(
            &poor,
            json_as(json!({ "type": "play", "instanceId": other.id, "zone": { "row": "units", "lane": 1 }, "playerId": "p1" })),
        );
        assert_eq!(then.players.p2.hero.health, poor.players.p2.hero.health - 1);
    }

    #[test]
    fn is_carried_into_the_step_a_prompt_re_enters_across_a_json_round_trip() {
        let mut state = playing("mana-before-pause", 4);
        let card = only(&in_hand(&mut state, &asking_runner().id, PlayerId::P1, 1));
        let paused = pb_act(
            &state,
            json_as(json!({ "type": "play", "instanceId": card.id, "zone": { "row": "units", "lane": 1 }, "playerId": "p1" })),
        );
        assert_eq!(paused.pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Mode));
        let round = round_trip(&paused);
        let answers: Vec<ActionBody> = legal_actions(&paused, PlayerId::P1)
            .into_iter()
            .filter(|action| action.action_type() == ActionType::Answer)
            .collect();
        let answer = only(&answers);
        let live = pb_act(&paused, json_as(with_player(&answer, PlayerId::P1)));
        let again = pb_act(&round, json_as(with_player(&answer, PlayerId::P1)));
        assert_eq!(hash_state(&again), hash_state(&live));
        assert_eq!(live.players.p2.hero.health, state.players.p2.hero.health - 4);
    }

    #[test]
    fn a_cast_records_its_casters_mana_as_the_cast_begins() {
        let mut state = playing("mana-before-cast", 5);
        let card = new_instance(&mut state, &runner().id, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        state.players.p1.hand.push(card.clone());
        let before = state.players.p2.hero.health;
        let mut bench = Bench::new(&state);
        cast_card(&mut bench.sink(&mut state), &card, Default::default());
        assert_eq!(state.players.p2.hero.health, before - 4);
        assert_eq!(state.players.p1.mana.current, 5);
    }
}
