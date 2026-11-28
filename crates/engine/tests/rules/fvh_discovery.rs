//! Felinors vs Humans, E4 (#5239): discovery, catalog filters and damage riders, each proved through a
//! fixture script that calls the verb or carries the flag (docs/felinors-vs-humans-set.md FV2, FV3): the
//! cards a run put into a hand (R5239000), a random summon lying Dormant (R5239001), the catalog's hook
//! and any-of filters (R5239002), damage split among the Units of both sides (R5239003), a keyword
//! Discover (R5239004), the dearest offer first (R5239005), a Flashback play with no Cry (R5239006),
//! damage that also gives Toxic (R5239007), past-version options (R5239008) and the simple cards
//! (R5239009).
//!
//! The file registers its own catalog on each test's thread (`install`): script-less (0) Spells for the
//! scenario's filler deck and every library, then the test's own cards. Ids are `fvhd-…`.

use jackioh_engine::effects;
use jackioh_engine::prelude::*;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

/// The script-less (0) Spell every library holds, so no turn ends by itself and no draw fatigues.
const FILLER: &str = "fvhd-filler-0";
/// A sturdy vanilla Unit for the field.
const WALL: &str = "stand-in (0) 1/9";
/// A script-less (0) Spell a resume step adds to its player's hand, carrying in its memory what the
/// step read.
const MARKER: &str = "fvhd-marker";
/// The memory key the marker carries the read under.
const READ_KEY: &str = "read";
/// The step every Discover here re-enters.
const PICKED_STEP: &str = "picked";

/// The split's total.
const SPLIT_DAMAGE: i32 = 4;
/// What the Flashback Spell's Cry deals the enemy hero.
const CRY_DAMAGE: i32 = 2;

/// A Core Common fixture definition, its Radiant face the same as its base.
fn def(id: &str, index: u32, type_: &str, cost: i32, stats: Option<(i32, i32)>, keywords: Value) -> CardDef {
    let face = match stats {
        Some((attack, health)) => json!({ "attack": attack, "health": health, "keywords": keywords, "text": id }),
        None => json!({ "keywords": keywords, "text": id }),
    };
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (FvH discovery fixture)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": face,
        "radiant": face,
    }))
}

fn unit(id: &str, index: u32, cost: i32, stats: (i32, i32)) -> CardDef {
    def(id, index, "Unit", cost, Some(stats), json!([]))
}

fn spell(id: &str, index: u32) -> CardDef {
    def(id, index, "Spell", 0, None, json!([]))
}

/// This file's catalog on the calling test's thread: `DECK_SIZE` script-less (0) Spells (the filler
/// deck and the libraries), the marker, then `cards`, each with its script on both faces.
fn install(cards: Vec<(CardDef, Script)>) {
    let mut catalog: CardDefs = IndexMap::new();
    for n in 0..DECK_SIZE {
        let filler = spell(&format!("fvhd-filler-{n}"), 525_900 + n as u32);
        catalog.insert(filler.id.clone(), filler);
    }
    let marker = spell(MARKER, 523_799);
    catalog.insert(marker.id.clone(), marker);
    let mut scripts: IndexMap<String, CardScripts> = IndexMap::new();
    for (card, script) in cards {
        scripts.insert(
            card.id.clone(),
            CardScripts {
                base: script.clone(),
                radiant: script,
            },
        );
        catalog.insert(card.id.clone(), card);
    }
    register_catalog(catalog);
    register_scripts(scripts);
}

/// p1's turn 9, its main phase; each side as given, with fillers to draw, and no turn that ends by itself.
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([FILLER, FILLER, FILLER, FILLER, FILLER, FILLER]);
        }
    }
    let mut s = scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

fn lane_id(s: &Scenario, player: PlayerId, lane: i32) -> String {
    s.unit_at(player, lane).id
}

fn hero_health(s: &Scenario, player: PlayerId) -> i32 {
    s.state().players[player].hero.health
}

/// A Spell whose Cry is `effect`, made fresh at each play.
fn cast(effect: impl Fn() -> Effect + Send + Sync + 'static) -> Script {
    Script {
        cry: Some(hook(move |_ctx| vec![effect()])),
        ..Script::default()
    }
}

/// The marker added to its player's hand with `read` in its memory.
fn mark(read: Value) -> Effect {
    effects::add_to_hand(json_as(json!({ "defId": MARKER, "memory": { READ_KEY: read } })))
}

/// What the marker p1 holds carries.
fn marked(s: &Scenario) -> Value {
    let marker = must(
        s.hand(P1).into_iter().find(|card| card.def_id == MARKER),
        "the marker the step added",
    );
    must(marker.memory.get(READ_KEY).cloned(), "the marker's read")
}

mod r5239000_added_to_hand_so_far {
    use super::*;

    const SOURCE: &str = "fvhd-source";
    const KEEPSAKE: &str = "fvhd-keepsake";
    const TRINKET: &str = "fvhd-trinket";

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (spell(SOURCE, 523_700), Script::default()),
            (spell(KEEPSAKE, 523_701), Script::default()),
            (spell(TRINKET, 523_702), Script::default()),
        ]
    }

    /// One run as p1's `source`: an add of each of `defs` to p1's hand, then the run's list.
    fn listed_after(s: &mut Scenario, source: &CardInstance, defs: &[&str]) -> Vec<String> {
        let mut events: Vec<GameEvent> = Vec::new();
        let state = s.state_mut();
        let mut rng = Rng::new(&state.seed.clone(), state.rng_cursor);
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let options = HookOptions {
            controller: Some(P1),
            ..Default::default()
        };
        let mut ctx = make_context(&mut sink, Some(source), options);
        for def_id in defs {
            (effects::add_to_hand(json_as(json!({ "defId": def_id }))).apply)(&mut ctx);
        }
        added_to_hand_so_far(&ctx)
    }

    fn held_ids(s: &Scenario) -> Vec<String> {
        s.hand(P1).into_iter().map(|card| card.id).collect()
    }

    #[test]
    fn r5239000_lists_the_cards_this_run_put_into_a_hand() {
        install(cards());
        let mut s = board("r5239000-run", json!({ "hand": [SOURCE, FILLER] }), json!({}));
        let source = s.card(SOURCE).clone();
        let before = held_ids(&s);
        let listed = listed_after(&mut s, &source, &[KEEPSAKE, TRINKET]);
        let fresh: Vec<CardInstance> = s
            .hand(P1)
            .into_iter()
            .filter(|card| !before.contains(&card.id))
            .collect();
        assert_eq!(fresh.len(), 2, "both adds landed");
        assert_eq!(listed.len(), 2, "{listed:?}");
        for card in &fresh {
            assert!(
                listed.iter().any(|entry| *entry == card.id || *entry == card.def_id),
                "{} is listed: {listed:?}",
                card.id
            );
        }
    }

    #[test]
    fn r5239000_another_runs_adds_and_the_cards_already_held_are_not_listed() {
        install(cards());
        let mut s = board("r5239000-other", json!({ "hand": [SOURCE, FILLER] }), json!({}));
        let source = s.card(SOURCE).clone();
        let first = listed_after(&mut s, &source, &[KEEPSAKE]);
        assert_eq!(first.len(), 1);
        let second = listed_after(&mut s, &source, &[]);
        assert!(second.is_empty(), "a new run starts with none: {second:?}");
    }
}

mod r5239001_summon_random_dormant {
    use super::*;

    const SLEEPER: &str = "fvhd-sleeper";
    const SUMMON_DORMANT: &str = "fvhd-summon-dormant";
    const SUMMON_AWAKE: &str = "fvhd-summon-awake";
    const DORMANT_TURNS: i32 = 2;

    fn summoner(dormant: bool) -> Script {
        cast(move || {
            let mut args: SummonRandomWithArgs = json_as(json!({ "query": { "defId": [SLEEPER] } }));
            if dormant {
                args.dormant_turns = Some(DORMANT_TURNS);
            }
            effects::summon_random_with(args)
        })
    }

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (unit(SLEEPER, 523_710, 1, (2, 2)), Script::default()),
            (spell(SUMMON_DORMANT, 523_711), summoner(true)),
            (spell(SUMMON_AWAKE, 523_712), summoner(false)),
        ]
    }

    fn summoned(seed: &str, summoner: &str) -> CardInstance {
        install(cards());
        let mut s = board(seed, json!({ "hand": [summoner] }), json!({}));
        s.play(summoner, json!({}));
        let card = s.unit_at(P1, 1);
        assert_eq!(card.def_id, SLEEPER);
        card
    }

    #[test]
    fn r5239001_summons_it_dormant_for_that_many_turns() {
        let card = summoned("r5239001-dormant", SUMMON_DORMANT);
        assert!(is_slumbering(&card));
        assert_eq!(slumber_turns_of(&card), Some(DORMANT_TURNS));
    }

    #[test]
    fn r5239001_absent_it_lands_awake() {
        let card = summoned("r5239001-awake", SUMMON_AWAKE);
        assert!(!is_slumbering(&card));
    }
}

mod r5239002_catalog_hooks_and_any_of {
    use super::*;

    const DIER: &str = "fvhd-dier";
    const AURA: &str = "fvhd-aura";
    const CRIER: &str = "fvhd-crier";
    const PLAIN: &str = "fvhd-plain";
    /// A Death on its Radiant face only.
    const RADIANT_DIER: &str = "fvhd-radiant-dier";

    fn dies() -> Script {
        Script {
            death: Some(hook(|_ctx| Vec::new())),
            ..Script::default()
        }
    }

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (unit(DIER, 523_720, 1, (1, 1)), dies()),
            (
                unit(AURA, 523_721, 1, (1, 1)),
                Script {
                    aura: Some(aura_hook(|_args| Vec::new())),
                    ..Script::default()
                },
            ),
            (
                spell(CRIER, 523_722),
                Script {
                    cry: Some(hook(|_ctx| Vec::new())),
                    ..Script::default()
                },
            ),
            (unit(PLAIN, 523_723, 1, (1, 1)), Script::default()),
            (unit(RADIANT_DIER, 523_724, 1, (1, 1)), Script::default()),
        ]
    }

    fn registered() {
        install(cards());
        let mut scripts = registered_scripts();
        scripts.insert(
            RADIANT_DIER.to_string(),
            CardScripts {
                base: Script::default(),
                radiant: dies(),
            },
        );
        register_scripts(scripts);
    }

    fn pool(args: CatalogQueryArgs) -> Vec<String> {
        let mut ids: Vec<String> = query(&args).into_iter().map(|def| def.id.clone()).collect();
        ids.sort();
        ids
    }

    fn sorted(ids: &[&str]) -> Vec<String> {
        let mut ids: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
        ids.sort();
        ids
    }

    fn with_hooks(hooks: Vec<HookFilter>) -> CatalogQueryArgs {
        CatalogQueryArgs {
            hooks: Some(hooks),
            ..CatalogQueryArgs::default()
        }
    }

    fn named(id: &str) -> CatalogQueryArgs {
        CatalogQueryArgs {
            def_id: Some(OneOrMany::One(id.to_string())),
            ..CatalogQueryArgs::default()
        }
    }

    #[test]
    fn r5239002_hooks_keeps_the_defs_whose_base_script_has_a_named_hook() {
        registered();
        assert_eq!(pool(with_hooks(vec![HookFilter::Death])), sorted(&[DIER]));
        assert_eq!(
            pool(with_hooks(vec![HookFilter::Aura, HookFilter::Cry])),
            sorted(&[AURA, CRIER]),
            "any of the named hooks"
        );
    }

    #[test]
    fn r5239002_a_hook_on_the_radiant_face_alone_or_no_script_is_not_kept() {
        registered();
        let deaths = pool(with_hooks(vec![HookFilter::Death]));
        assert!(!deaths.contains(&RADIANT_DIER.to_string()), "{deaths:?}");
        assert!(!deaths.contains(&PLAIN.to_string()), "{deaths:?}");
        assert!(!deaths.contains(&FILLER.to_string()), "{deaths:?}");
    }

    #[test]
    fn r5239002_any_of_matches_when_any_sub_query_matches() {
        registered();
        let either = CatalogQueryArgs {
            any_of: Some(vec![named(PLAIN), with_hooks(vec![HookFilter::Death])]),
            ..CatalogQueryArgs::default()
        };
        assert_eq!(pool(either), sorted(&[DIER, PLAIN]));
    }

    #[test]
    fn r5239002_any_of_with_no_sub_query_matching_is_empty() {
        registered();
        let nothing = CatalogQueryArgs {
            any_of: Some(vec![named("fvhd-no-such-card"), named("fvhd-no-such-card-either")]),
            ..CatalogQueryArgs::default()
        };
        assert!(pool(nothing).is_empty());
    }
}

mod r5239003_damage_split_among_units {
    use super::*;

    const SPLIT: &str = "fvhd-split-units";

    fn cards() -> Vec<(CardDef, Script)> {
        vec![(
            spell(SPLIT, 523_730),
            cast(|| {
                let mut args: DamageSplitArgs = json_as(json!({ "amount": SPLIT_DAMAGE }));
                args.among = Some(SplitAmong::Units);
                effects::damage_split(args)
            }),
        )]
    }

    /// p1 splits among the board `p1` and `p2` set up; returns the scenario and both heroes' health
    /// before.
    fn split(seed: &str, p1_field: Value, p2_field: Value) -> (Scenario, i32, i32) {
        install(cards());
        let mut s = board(
            seed,
            json!({ "hand": [SPLIT], "field": p1_field }),
            json!({ "field": p2_field }),
        );
        let heroes = (hero_health(&s, P1), hero_health(&s, P2));
        s.play(SPLIT, json!({}));
        (s, heroes.0, heroes.1)
    }

    fn damage_on(s: &Scenario, id: &str) -> i32 {
        s.card(id).damage
    }

    #[test]
    fn r5239003_a_unit_of_its_own_side_is_hit() {
        let (s, _, _) = split("r5239003-own", json!([WALL]), json!([]));
        assert_eq!(damage_on(&s, &lane_id(&s, P1, 1)), SPLIT_DAMAGE);
    }

    #[test]
    fn r5239003_a_unit_of_the_enemy_side_is_hit() {
        let (s, _, _) = split("r5239003-enemy", json!([]), json!([WALL]));
        assert_eq!(damage_on(&s, &lane_id(&s, P2, 1)), SPLIT_DAMAGE);
    }

    #[test]
    fn r5239003_never_a_hero() {
        let (s, p1, p2) = split("r5239003-heroes", json!([WALL]), json!([WALL]));
        assert_eq!(hero_health(&s, P1), p1);
        assert_eq!(hero_health(&s, P2), p2);
        let dealt = damage_on(&s, &lane_id(&s, P1, 1)) + damage_on(&s, &lane_id(&s, P2, 1));
        assert_eq!(dealt, SPLIT_DAMAGE, "every hit landed on a Unit");
    }

    #[test]
    fn r5239003_with_no_unit_standing_no_hero_is_hit() {
        let (s, p1, p2) = split("r5239003-empty", json!([]), json!([]));
        assert_eq!(hero_health(&s, P1), p1);
        assert_eq!(hero_health(&s, P2), p2);
    }
}

mod r5239004_discover_keyword {
    use super::*;

    /// "Discover one of Taunt, Rush and Charge your chosen Unit lacks."
    const DISCOVER_LACKING: &str = "fvhd-discover-lacking";
    /// "Discover one of two of Taunt, Rush, Charge and Lifesteal."
    const DISCOVER_TWO: &str = "fvhd-discover-two";
    const TAUNTER: &str = "stand-in (0) 1/5 Taunt";

    /// The resume step: the marker with what `chosen_options` read.
    fn picked() -> IndexMap<&'static str, Hook> {
        IndexMap::from([(
            PICKED_STEP,
            hook(|ctx| vec![mark(json!(chosen_options(ctx)))]),
        )])
    }

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (
                spell(DISCOVER_LACKING, 523_740),
                Script {
                    targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
                    cry: Some(hook(|_ctx| {
                        vec![effects::choose::discover_keyword(DiscoverKeywordArgs {
                            step: PICKED_STEP.to_string(),
                            pool: Some(vec![Keyword::Taunt, Keyword::Rush, Keyword::Charge]),
                            count: Some(3),
                            lacking_on: Some(TargetSpec::Chosen { index: None }),
                            data: None,
                        })]
                    })),
                    resume: picked(),
                    ..Script::default()
                },
            ),
            (
                spell(DISCOVER_TWO, 523_741),
                Script {
                    cry: Some(hook(|_ctx| {
                        vec![effects::choose::discover_keyword(DiscoverKeywordArgs {
                            step: PICKED_STEP.to_string(),
                            pool: Some(vec![
                                Keyword::Taunt,
                                Keyword::Rush,
                                Keyword::Charge,
                                Keyword::Lifesteal,
                            ]),
                            count: Some(2),
                            lacking_on: None,
                            data: None,
                        })]
                    })),
                    resume: picked(),
                    ..Script::default()
                },
            ),
        ]
    }

    #[test]
    fn r5239004_offers_count_keywords() {
        install(cards());
        let mut s = board("r5239004-count", json!({ "hand": [DISCOVER_TWO] }), json!({}));
        s.play(DISCOVER_TWO, json!({}));
        let modes = s.prompt_modes();
        assert_eq!(modes.len(), 2, "{modes:?}");
    }

    #[test]
    fn r5239004_never_offers_a_keyword_the_target_already_has() {
        install(cards());
        let mut s = board(
            "r5239004-lacking",
            json!({ "hand": [DISCOVER_LACKING], "field": [TAUNTER] }),
            json!({}),
        );
        let taunter = lane_id(&s, P1, 1);
        s.play(DISCOVER_LACKING, pick::targets(pick::id(&taunter)));
        let modes = s.prompt_modes();
        assert_eq!(modes.len(), 2, "Taunt is held, so two of the three are left: {modes:?}");
    }

    #[test]
    fn r5239004_the_pick_is_read_back() {
        install(cards());
        let mut s = board("r5239004-read", json!({ "hand": [DISCOVER_TWO] }), json!({}));
        s.play(DISCOVER_TWO, json!({}));
        let modes = s.prompt_modes();
        let answer = must(modes.first().cloned(), "an option");
        s.answer(pick::mode(&answer));
        let read = marked(&s);
        assert!(
            read.as_array().is_some_and(|picks| picks.contains(&json!(answer))),
            "chosen_options read {read} for {answer}"
        );
    }
}

mod r5239005_dearest_first {
    use super::*;

    const ONE: &str = "fvhd-cost-1";
    const TWO: &str = "fvhd-cost-2";
    const THREE: &str = "fvhd-cost-3";
    const FOUR: &str = "fvhd-cost-4";
    /// A (9) Unit whose Cry discovers the dearest of the pool and itself.
    const DEAREST_UNIT: &str = "fvhd-dearest-unit";
    const DEAREST_THREE: &str = "fvhd-dearest-three";
    const DEAREST_TWO: &str = "fvhd-dearest-two";

    fn discover(count: i32, also: &'static [&'static str]) -> Script {
        Script {
            cry: Some(hook(move |_ctx| {
                let mut pool: Vec<&str> = vec![ONE, TWO, THREE, FOUR];
                pool.extend(also);
                let mut args: DiscoverFromCatalogArgs = json_as(json!({
                    "step": PICKED_STEP,
                    "query": { "defId": pool },
                    "count": count,
                }));
                args.order = Some(DiscoverOrder::Dearest);
                vec![effects::discover_from_catalog(args)]
            })),
            ..Script::default()
        }
    }

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (unit(ONE, 523_750, 1, (1, 1)), Script::default()),
            (unit(TWO, 523_751, 2, (1, 1)), Script::default()),
            (unit(THREE, 523_752, 3, (1, 1)), Script::default()),
            (unit(FOUR, 523_753, 4, (1, 1)), Script::default()),
            (unit(DEAREST_UNIT, 523_754, 9, (1, 1)), discover(3, &[DEAREST_UNIT])),
            (spell(DEAREST_THREE, 523_755), discover(3, &[])),
            (spell(DEAREST_TWO, 523_756), discover(2, &[])),
        ]
    }

    /// p1 plays `discoverer`, held as `held` (its hand entry); the options offered, in order.
    fn offered(seed: &str, held: Value, discoverer: &str) -> Vec<String> {
        install(cards());
        let mut s = board(seed, json!({ "hand": [held] }), json!({}));
        s.play(discoverer, json!({}));
        s.prompt_modes()
    }

    #[test]
    fn r5239005_offers_the_dearest_options_first() {
        assert_eq!(
            offered("r5239005-three", json!(DEAREST_THREE), DEAREST_THREE),
            vec![FOUR.to_string(), THREE.to_string(), TWO.to_string()]
        );
    }

    #[test]
    fn r5239005_the_cheaper_options_are_left_out() {
        let modes = offered("r5239005-two", json!(DEAREST_TWO), DEAREST_TWO);
        assert_eq!(modes, vec![FOUR.to_string(), THREE.to_string()]);
        assert!(!modes.contains(&ONE.to_string()) && !modes.contains(&TWO.to_string()));
    }

    #[test]
    fn r5239005_the_running_card_is_never_offered_even_when_in_the_pool() {
        // Printed (9), the dearest of the pool, it plays for (0) here.
        let held = json!({ "def": DEAREST_UNIT, "costOverride": 0 });
        let modes = offered("r5239005-self", held, DEAREST_UNIT);
        assert!(!modes.contains(&DEAREST_UNIT.to_string()), "{modes:?}");
        assert_eq!(modes, vec![FOUR.to_string(), THREE.to_string(), TWO.to_string()]);
    }
}

mod r5239006_flashback_without_cry {
    use super::*;

    /// A (1) Spell with Flashback: "Deal 2 damage to the enemy hero", whose Flashback play runs no Cry.
    const ECHO_QUIET: &str = "fvhd-echo-quiet";
    /// The same Spell without the flag.
    const ECHO_LOUD: &str = "fvhd-echo-loud";

    fn echo(quiet: bool) -> Script {
        Script {
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": CRY_DAMAGE }),
                ))]
            })),
            static_flags: Some(StaticFlags {
                flashback_without_cry: quiet.then_some(true),
                ..StaticFlags::default()
            }),
            ..Script::default()
        }
    }

    fn cards() -> Vec<(CardDef, Script)> {
        let flashback = json!([{ "kind": "Flashback" }]);
        vec![
            (def(ECHO_QUIET, 523_760, "Spell", 1, None, flashback.clone()), echo(true)),
            (def(ECHO_LOUD, 523_761, "Spell", 1, None, flashback), echo(false)),
        ]
    }

    /// p1 plays `card` from hand, then again from the graveyard by its Flashback; returns the scenario
    /// and the enemy hero's health between the two plays.
    fn played_twice(seed: &str, card: &str) -> (Scenario, i32) {
        install(cards());
        let mut s = board(seed, json!({ "hand": [card, FILLER] }), json!({}));
        let health = hero_health(&s, P2);
        s.play(card, json!({}));
        assert_eq!(hero_health(&s, P2), health - CRY_DAMAGE, "a play from hand runs its Cry");
        let between = hero_health(&s, P2);
        let in_graveyard = must(
            s.pile(P1, "graveyard").into_iter().find(|landed| landed.def_id == card),
            "the Spell in the graveyard",
        );
        let body: ActionBody = json_as(json!({ "type": "play", "instanceId": in_graveyard.id }));
        s.act(body, P1, "a Flashback play from the graveyard");
        assert!(
            s.last_events().iter().any(|event| matches!(
                event,
                GameEvent::CardPlayed { instance_id, from: Some(PlayedFrom::Graveyard), .. }
                    if *instance_id == in_graveyard.id
            )),
            "it was played from the graveyard"
        );
        (s, between)
    }

    #[test]
    fn r5239006_a_play_that_spends_flashback_runs_no_cry() {
        let (s, between) = played_twice("r5239006-quiet", ECHO_QUIET);
        assert_eq!(hero_health(&s, P2), between);
    }

    #[test]
    fn r5239006_absent_the_flashback_play_runs_its_cry() {
        let (s, between) = played_twice("r5239006-loud", ECHO_LOUD);
        assert_eq!(hero_health(&s, P2), between - CRY_DAMAGE);
    }
}

mod r5239007_damage_also_toxic {
    use super::*;

    /// A 3/20 Unit whose hits also give Toxic.
    const VENOM: &str = "fvhd-venom";
    /// The same numbers with no rider.
    const PLAIN_STRIKER: &str = "stand-in (0) 3/20";
    const TARGET: &str = "stand-in (0) 1/10";
    const FRAIL: &str = "stand-in (0) 1/2";
    const SHIELDED: &str = "stand-in (0) 1/10 Divine Shield";

    fn cards() -> Vec<(CardDef, Script)> {
        vec![(
            unit(VENOM, 523_770, 0, (3, 20)),
            Script {
                static_flags: Some(StaticFlags {
                    damage_also_toxic: Some(true),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            },
        )]
    }

    /// `attacker` (p1's lane 1) attacks p2's lane-1 `target`; returns the scenario and the target's id.
    fn struck(seed: &str, attacker: &str, target: &str) -> (Scenario, String) {
        install(cards());
        let mut s = board(seed, json!({ "field": [attacker] }), json!({ "field": [target] }));
        let (striker, defender) = (lane_id(&s, P1, 1), lane_id(&s, P2, 1));
        s.attack(striker.as_str(), defender.as_str());
        (s, defender)
    }

    fn toxic_events(s: &Scenario) -> usize {
        s.last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::CounterChanged { counter: CounterKind::Toxic, .. }))
            .count()
    }

    #[test]
    fn r5239007_each_hit_also_gives_the_surviving_target_toxic_equal_to_what_it_dealt() {
        let (s, target) = struck("r5239007-toxic", VENOM, TARGET);
        assert_eq!(s.card(target.as_str()).damage, 3);
        assert_eq!(query::toxic_on(s.card(target.as_str())), 3);
    }

    #[test]
    fn r5239007_a_target_the_hit_kills_takes_no_toxic() {
        let (mut s, target) = struck("r5239007-kill", VENOM, FRAIL);
        s.expect_in_zone(target.as_str(), "graveyard");
        assert_eq!(toxic_events(&s), 0);
    }

    #[test]
    fn r5239007_a_hit_that_dealt_nothing_gives_no_toxic() {
        let (s, target) = struck("r5239007-shield", VENOM, SHIELDED);
        assert_eq!(s.card(target.as_str()).damage, 0);
        assert_eq!(query::toxic_on(s.card(target.as_str())), 0);
    }

    #[test]
    fn r5239007_absent_a_hit_gives_no_toxic() {
        let (s, target) = struck("r5239007-plain", PLAIN_STRIKER, TARGET);
        assert_eq!(s.card(target.as_str()).damage, 3);
        assert_eq!(query::toxic_on(s.card(target.as_str())), 0);
    }
}

mod r5239008_past_options {
    use super::*;

    const ALPHA: &str = "fvhd-alpha";
    const BETA: &str = "fvhd-beta";
    const GAMMA: &str = "fvhd-gamma";
    const DISCOVER_PAST: &str = "fvhd-discover-past";
    const DISCOVER_NOW: &str = "fvhd-discover-now";
    const PAST_VERSION: &str = "v1";
    const PAST_COUNT: i32 = 1;

    fn discoverer(past: bool) -> Script {
        Script {
            cry: Some(hook(move |_ctx| {
                let mut args: DiscoverFromCatalogArgs = json_as(json!({
                    "step": PICKED_STEP,
                    "query": { "defId": [ALPHA, BETA, GAMMA] },
                }));
                if past {
                    args.past_options = Some(PAST_COUNT);
                }
                vec![effects::discover_from_catalog(args)]
            })),
            resume: IndexMap::from([(
                PICKED_STEP,
                hook(|ctx| vec![mark(json!(discovered_card(ctx)))]),
            )]),
            ..Script::default()
        }
    }

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (unit(ALPHA, 523_780, 1, (1, 1)), Script::default()),
            (unit(BETA, 523_781, 2, (2, 2)), Script::default()),
            (unit(GAMMA, 523_782, 3, (3, 3)), Script::default()),
            (spell(DISCOVER_PAST, 523_783), discoverer(true)),
            (spell(DISCOVER_NOW, 523_784), discoverer(false)),
        ]
    }

    /// Each pool card's one past version, a 9/9.
    fn past_defs() -> Vec<CardDef> {
        [ALPHA, BETA, GAMMA]
            .iter()
            .map(|id| {
                let mut past = must(registered_catalog().get(*id).cloned(), "the card");
                past.id = format!("{id}{PAST_ID_MARK}{PAST_VERSION}");
                past.base.attack = Some(9);
                past.base.health = Some(9);
                past
            })
            .collect()
    }

    fn offered(seed: &str, discoverer: &str) -> Scenario {
        install(cards());
        register_past_defs(past_defs());
        let mut s = board(seed, json!({ "hand": [discoverer] }), json!({}));
        s.play(discoverer, json!({}));
        s
    }

    fn past_modes(modes: &[String]) -> Vec<String> {
        modes
            .iter()
            .filter(|mode| split_past_id(mode).is_some())
            .cloned()
            .collect()
    }

    #[test]
    fn r5239008_offers_that_many_past_version_options() {
        let s = offered("r5239008-past", DISCOVER_PAST);
        let modes = s.prompt_modes();
        assert_eq!(past_modes(&modes).len(), PAST_COUNT as usize, "{modes:?}");
    }

    #[test]
    fn r5239008_absent_no_past_version_is_offered() {
        let s = offered("r5239008-now", DISCOVER_NOW);
        let modes = s.prompt_modes();
        assert!(!modes.is_empty());
        assert!(past_modes(&modes).is_empty(), "{modes:?}");
    }

    #[test]
    fn r5239008_discovered_card_reads_the_pick_a_past_version_included() {
        let mut s = offered("r5239008-read-past", DISCOVER_PAST);
        let answer = must(past_modes(&s.prompt_modes()).pop(), "the past option");
        s.answer(pick::mode(&answer));
        assert_eq!(marked(&s), json!(answer));

        let mut s = offered("r5239008-read-now", DISCOVER_NOW);
        let answer = must(s.prompt_modes().pop(), "an option");
        s.answer(pick::mode(&answer));
        assert_eq!(marked(&s), json!(answer));
    }
}

mod r5239009_is_simple {
    use super::*;

    const SHORT: &str = "fvhd-short";
    const EDGE: &str = "fvhd-edge";
    const LONG: &str = "fvhd-long";

    fn registered() {
        install(vec![
            (spell(SHORT, 523_790), Script::default()),
            (spell(EDGE, 523_791), Script::default()),
            (spell(LONG, 523_792), Script::default()),
        ]);
        register_code_lines(vec![
            (SHORT, 3),
            (EDGE, SIMPLE_CODE_LINES),
            (LONG, SIMPLE_CODE_LINES + 1),
        ]);
    }

    #[test]
    fn r5239009_a_card_with_at_most_simple_code_lines_is_simple() {
        assert_eq!(SIMPLE_CODE_LINES, 10);
        registered();
        assert!(is_simple(SHORT));
        assert!(is_simple(EDGE), "exactly SIMPLE_CODE_LINES is still simple");
    }

    #[test]
    fn r5239009_a_card_with_more_is_not() {
        registered();
        assert!(!is_simple(LONG));
    }
}
