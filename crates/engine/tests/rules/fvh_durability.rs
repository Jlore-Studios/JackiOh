//! E1 #5236's Durability X (docs/felinors-vs-humans-set.md FV2 "Durability X", FV3 E1; R5236000 to
//! R5236006): one counter per card shared by its Activates, what spends it and what never does, the
//! refusal and the view at 0, a Field Spell's plain move to its graveyard, Refresh and a countered
//! Activate, the reset on leaving the field, Buff and Nerf, and Core #79 Twinspell's base face.
//!
//! Proved on this file's own fixtures (`fvh-dur-` ids, indexed from 5236100) through the testkit's
//! `scenario()`, and on the real Twinspell (`jackioh_cards::register_all`). Durability is printed on a
//! fixture's faces as `Keyword::Durability { n }`, never given, so no test leans on how a given one counts.

use jackioh_engine::effects::{
    ActivationOfArgs, TuneDirection, bounce, damage, gain_mana, refresh_activates, run_activation, tune_once,
};
use jackioh_engine::subsystems::activate::{
    ability_uses_this_turn, activation_views_for, durability_left, spend_durability, why_cannot_activate_ability,
};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{apply_now, both, by, live, run_on_sink, setup_catalog};

/// A (0) 1/5 Unit with no text: libraries and hands, so no turn ends by itself.
const PLAIN: &str = "fvh-dur-plain";
/// A Unit, Durability 3, with two Activates, "left" and "right", each once a turn: gain 1 mana.
const SHARED: &str = "fvh-dur-shared";
/// A Unit, Durability 2, "Activate ♾️: gain 1 mana": only its Durability can refuse it.
const MANY: &str = "fvh-dur-many";
/// A Unit, Durability 3, "Activate 2: gain 1 mana".
const PAIR: &str = "fvh-dur-pair";
/// A Unit, Durability 2, "Activate: gain 1 mana" (once a turn).
const ONCE: &str = "fvh-dur-once";
/// A Unit, Durability 1, "Activate: gain 1 mana".
const SINGLE: &str = "fvh-dur-single";
/// A Unit with no Durability, "Activate ♾️: gain 1 mana".
const BARE: &str = "fvh-dur-bare";
/// A Field Spell, Durability 1, "Activate: gain 1 mana. Death: deal 5 damage to the enemy hero."
const FIELD: &str = "fvh-dur-field";
/// `FIELD` with Indestructible.
const WARDED: &str = "fvh-dur-warded";
/// `FIELD` at Durability 2.
const LASTING: &str = "fvh-dur-lasting";
/// A Cloaked Unit, Durability 2, "Activate: gain 1 mana".
const CLOAKED: &str = "fvh-dur-cloaked";
/// A Field Trap that counters every enemy Activate (R3550005's Radiant shape).
const COUNTER: &str = "fvh-dur-counter";

const SHARED_DURABILITY: i32 = 3;
const MANY_DURABILITY: i32 = 2;
const PAIR_DURABILITY: i32 = 3;
const PAIR_USES: i32 = 2;
const ONCE_DURABILITY: i32 = 2;
const SINGLE_DURABILITY: i32 = 1;
const FIELD_DURABILITY: i32 = 1;
const LASTING_DURABILITY: i32 = 2;
const CLOAKED_DURABILITY: i32 = 2;
/// The Durability the Starship test prints on the Dormant Starship's own definition.
const SHIP_DURABILITY: i32 = 2;

const UNIT_ATTACK: i32 = 1;
const UNIT_HEALTH: i32 = 5;
/// What each fixture Activate gains.
const GAIN: i32 = 1;
/// What a fixture Field Spell's Death would deal the enemy hero, were it run.
const DEATH_HIT: i32 = 5;
/// The id of every once-a-turn and unlimited fixture ability.
const GAIN_ABILITY: &str = "gain";
const LEFT: &str = "left";
const RIGHT: &str = "right";
/// The engine's own ability of a card set by its Cloaked (R783006).
const REVEAL: &str = "reveal";
/// The backrow lane a Cloaked card is set in.
const SET_LANE: i32 = 2;
/// FV2's refusal at 0, "No Durability left", matched without its first word's case.
const NO_DURABILITY: &str = "Durability left";
/// How many rng cursors a Buff or a Nerf is drawn at (`after_one`).
const SAMPLES: u32 = 60;
/// How many Nerfs in a row the floor test applies.
const NERFS_IN_A_ROW: usize = 40;

// Fixtures

fn card(index: u32, id: &str, type_: &str, printed: Value, durability: Option<i32>) -> CardDef {
    let is_unit = type_ == "Unit";
    let face = |scale: i32| {
        if is_unit {
            json!({
                "attack": UNIT_ATTACK * scale,
                "health": UNIT_HEALTH * scale,
                "keywords": printed.clone(),
                "text": id,
            })
        } else {
            json!({ "keywords": printed.clone(), "text": id })
        }
    };
    let mut def: CardDef = json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (Durability fixture)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face(1),
        "radiant": face(2),
    }));
    if let Some(n) = durability {
        def.base.keywords.push(Keyword::Durability { n });
        def.radiant.keywords.push(Keyword::Durability { n });
    }
    def
}

fn defs() -> Vec<CardDef> {
    vec![
        card(5236100, PLAIN, "Unit", json!([]), None),
        card(5236101, SHARED, "Unit", json!([]), Some(SHARED_DURABILITY)),
        card(5236102, MANY, "Unit", json!([]), Some(MANY_DURABILITY)),
        card(5236103, PAIR, "Unit", json!([]), Some(PAIR_DURABILITY)),
        card(5236104, ONCE, "Unit", json!([]), Some(ONCE_DURABILITY)),
        card(5236105, SINGLE, "Unit", json!([]), Some(SINGLE_DURABILITY)),
        card(5236106, BARE, "Unit", json!([]), None),
        card(5236107, FIELD, "Field Spell", json!([]), Some(FIELD_DURABILITY)),
        card(
            5236108,
            WARDED,
            "Field Spell",
            json!([{ "kind": "Indestructible" }]),
            Some(FIELD_DURABILITY),
        ),
        card(5236109, LASTING, "Field Spell", json!([]), Some(LASTING_DURABILITY)),
        card(
            5236110,
            CLOAKED,
            "Unit",
            json!([{ "kind": "Cloaked" }]),
            Some(CLOAKED_DURABILITY),
        ),
        card(5236111, COUNTER, "Field Trap", json!([]), None),
    ]
}

fn gain() -> Hook {
    hook(|_ctx| vec![gain_mana(json_as(json!({ "amount": GAIN })))])
}

fn ability(id: &str, uses: ActivationUses) -> ActivationDecl {
    ActivationDecl {
        id: id.to_string(),
        label: id.to_string(),
        uses,
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: gain(),
    }
}

fn once_a_turn() -> Vec<ActivationDecl> {
    vec![ability(GAIN_ABILITY, ActivationUses::Count(1))]
}

fn activates(abilities: Vec<ActivationDecl>) -> CardScripts {
    both(Script {
        activations: abilities,
        ..Script::default()
    })
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut all: IndexMap<String, CardScripts> = IndexMap::new();
    all.insert(
        SHARED.to_string(),
        activates(vec![
            ability(LEFT, ActivationUses::Count(1)),
            ability(RIGHT, ActivationUses::Count(1)),
        ]),
    );
    for id in [MANY, BARE] {
        all.insert(
            id.to_string(),
            activates(vec![ability(GAIN_ABILITY, ActivationUses::Unlimited)]),
        );
    }
    all.insert(
        PAIR.to_string(),
        activates(vec![ability(GAIN_ABILITY, ActivationUses::Count(PAIR_USES))]),
    );
    for id in [ONCE, SINGLE, CLOAKED] {
        all.insert(id.to_string(), activates(once_a_turn()));
    }
    for id in [FIELD, WARDED, LASTING] {
        all.insert(
            id.to_string(),
            both(Script {
                activations: once_a_turn(),
                death: Some(hook(|_ctx| {
                    vec![damage(json_as(
                        json!({ "to": { "of": "enemyHero" }, "amount": DEATH_HIT }),
                    ))]
                })),
                ..Script::default()
            }),
        );
    }
    all.insert(
        COUNTER.to_string(),
        both(Script {
            static_flags: Some(json_as(json!({ "countersRangedAll": true }))),
            ..Script::default()
        }),
    );
    all
}

/// The engine's fixture catalog (`harness::setup_catalog`) with this file's cards and scripts on top.
fn register() {
    setup_catalog();
    let mut catalog: CardDefs = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(scripts());
    register_scripts(merged);
}

/// p1's turn 9, its main phase; each side's setup as given, plain cards to draw and to hold, and no
/// turn that ends by itself (R345).
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    register();
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([PLAIN, PLAIN, PLAIN, PLAIN]);
        }
        if side.get("hand").is_none() {
            side["hand"] = json!([PLAIN]);
        }
    }
    let mut s = scenario(json!({ "seed": seed, "active": "p1", "p1": p1, "p2": p2 }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

// Reads

fn left(s: &Scenario, card: &str) -> Option<i32> {
    durability_left(s.state(), s.card(card))
}

fn mana(s: &Scenario, player: PlayerId) -> i32 {
    s.state().players[player].mana.current
}

fn hero_health(s: &Scenario, player: PlayerId) -> i32 {
    s.state().players[player].hero.health
}

fn hand_id(s: &Scenario, def: &str) -> String {
    s.hand(P1)
        .into_iter()
        .find(|card| card.def_id == def)
        .unwrap_or_else(|| panic!("setup: no {def} in p1's hand"))
        .id
}

fn backrow_id(s: &Scenario, player: PlayerId, lane: i32) -> String {
    s.backrow(player, lane)
        .unwrap_or_else(|| panic!("setup: nothing in {player:?}'s backrow lane {lane}"))
        .id
}

/// Why its controller may not use the card's ability now (`why_cannot_activate_ability`), or `None`.
fn refusal(s: &Scenario, card: &str, ability: Option<&str>) -> Option<String> {
    let player = s.card(card).controller;
    why_cannot_activate_ability(s.state(), player, card, ability)
        .err()
        .map(|error| error.message)
}

/// The card's abilities as its controller's view shows them.
fn views(s: &Scenario, card: &str) -> Vec<ActivationView> {
    let live_card = s.card(card);
    activation_views_for(s.state(), live_card.controller, live_card).unwrap_or_default()
}

/// The `counterChanged` events of the Durability counter on `card` among `events`.
fn durability_counts(events: &[GameEvent], card: &str) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(
                event,
                GameEvent::CounterChanged { instance_id, counter: CounterKind::Durability, .. }
                    if instance_id == card
            )
        })
        .count()
}

fn destroyed(events: &[GameEvent], card: &str) -> bool {
    events
        .iter()
        .any(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if instance_id == card))
}

fn run_of(card: &str, spend: bool) -> Effect {
    run_activation(ActivationOfArgs {
        instance_id: Some(card.to_string()),
        spend_use: spend.then_some(true),
        ..ActivationOfArgs::default()
    })
}

/// One Nerf or Buff applied to `card` for p1, as R386's draw makes it, the rng cursor written back.
fn tuned_once(state: &mut GameState, card: &str, direction: TuneDirection) {
    let target = live(state, card);
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(&mut sink, None, by(P1));
        tune_once(&mut ctx, &target, direction, true);
    }
    state.rng_cursor = rng.cursor();
}

/// The card's Durability left after one Nerf or Buff drawn at each of `SAMPLES` rng cursors, each from
/// the scenario's state as it stands.
fn after_one(s: &Scenario, card: &str, direction: TuneDirection) -> Vec<Option<i32>> {
    (0..SAMPLES)
        .map(|cursor| {
            let mut state = s.state().clone();
            state.rng_cursor = cursor;
            tuned_once(&mut state, card, direction);
            durability_left(&state, &live(&state, card))
        })
        .collect()
}

mod r5236000_one_counter_spent_by_each_use {
    use super::*;

    #[test]
    fn r5236000_its_activates_share_one_counter_and_each_activate_action_spends_1() {
        let mut s = board("r5236000-shared", json!({ "field": [SHARED] }), json!({}));
        let id = s.unit_at(P1, 1).id;
        assert_eq!(left(&s, &id), Some(SHARED_DURABILITY));
        s.activate(id.as_str(), json!({ "ability": LEFT }));
        assert_eq!(left(&s, &id), Some(SHARED_DURABILITY - 1));
        assert_eq!(
            durability_counts(s.last_events(), &id),
            1,
            "the use is reported on the Durability counter"
        );
        // The other Activate spends the same counter, not one of its own.
        s.activate(id.as_str(), json!({ "ability": RIGHT }));
        assert_eq!(left(&s, &id), Some(SHARED_DURABILITY - 2));
        let shown: Vec<Option<i32>> = views(&s, &id).iter().map(|view| view.durability_left).collect();
        assert_eq!(
            shown,
            vec![Some(SHARED_DURABILITY - 2), Some(SHARED_DURABILITY - 2)],
            "both abilities show the one count"
        );
    }

    #[test]
    fn r5236000_a_run_with_spend_use_spends_exactly_1_even_of_an_activate_2() {
        let mut s = board("r5236000-spend-use", json!({ "field": [PAIR] }), json!({}));
        let id = s.unit_at(P1, 1).id;
        let before = mana(&s, P1);
        apply_now(s.state_mut(), run_of(&id, true));
        assert_eq!(mana(&s, P1), before + GAIN, "the ability ran");
        // R2521003 spends every use it has this turn; its Durability takes exactly 1.
        assert_eq!(left(&s, &id), Some(PAIR_DURABILITY - 1));
    }

    #[test]
    fn r5236000_a_run_without_spend_use_is_no_use_and_spends_none() {
        let mut s = board("r5236000-free-run", json!({ "field": [ONCE] }), json!({}));
        let id = s.unit_at(P1, 1).id;
        let before = mana(&s, P1);
        apply_now(s.state_mut(), run_of(&id, false));
        assert_eq!(mana(&s, P1), before + GAIN, "the ability ran");
        assert_eq!(left(&s, &id), Some(ONCE_DURABILITY));
        assert_eq!(s.card(id.as_str()).durability_spent.unwrap_or(0), 0);
    }

    #[test]
    fn r5236000_cloakeds_reveal_spends_none() {
        let mut s = board(
            "r5236000-reveal",
            json!({ "hand": [CLOAKED, PLAIN] }),
            json!({}),
        );
        let id = hand_id(&s, CLOAKED);
        s.play(
            id.as_str(),
            json!({ "zone": SET_LANE, "row": "backrow", "faceDown": "cloaked" }),
        );
        s.activate(id.as_str(), json!({ "ability": REVEAL }));
        assert_eq!(
            s.card(id.as_str()).face_up,
            Some(true),
            "setup: the reveal brought it out"
        );
        s.expect_in_zone(id.as_str(), "field");
        assert_eq!(durability_counts(s.last_events(), &id), 0);
        assert_eq!(s.card(id.as_str()).durability_spent.unwrap_or(0), 0);
        assert_eq!(left(&s, &id), Some(CLOAKED_DURABILITY));
    }

    #[test]
    fn r5236000_a_card_without_the_keyword_has_none_left_and_its_uses_count_nothing() {
        let mut s = board("r5236000-bare", json!({ "field": [BARE] }), json!({}));
        let id = s.unit_at(P1, 1).id;
        assert_eq!(left(&s, &id), None);
        s.activate(id.as_str(), json!({}));
        s.activate(id.as_str(), json!({}));
        assert_eq!(durability_counts(s.events(), &id), 0);
        assert_eq!(left(&s, &id), None);
        assert_eq!(refusal(&s, &id, None), None);
        assert!(views(&s, &id).iter().all(|view| view.durability_left.is_none()));
    }

    #[test]
    fn r5236000_spend_durability_spends_1_and_reports_the_durability_counter() {
        let mut s = board("r5236000-spend", json!({ "field": [ONCE] }), json!({}));
        let id = s.unit_at(P1, 1).id;
        let (_, events) = run_on_sink(s.state_mut(), |sink| spend_durability(sink, &id));
        assert_eq!(durability_counts(&events, &id), 1);
        assert_eq!(left(&s, &id), Some(ONCE_DURABILITY - 1));
    }

    mod a_starship_launch {
        use super::*;
        use crate::rules::fixtures::combat::plain;
        use crate::rules::fixtures::starship::{bolt, booster, hull, register_starship_fixtures};

        /// p1's booster killed by a bolt, so p1 holds a Dormant Starship whose definition prints
        /// Durability `SHIP_DURABILITY`.
        fn built_ship() -> (Scenario, CardInstance) {
            register_starship_fixtures();
            let mut catalog: CardDefs = registered_catalog().clone();
            let mut ship_def = hull.clone();
            ship_def.base.keywords.push(Keyword::Durability { n: SHIP_DURABILITY });
            ship_def.radiant.keywords.push(Keyword::Durability { n: SHIP_DURABILITY });
            catalog.insert(ship_def.id.clone(), ship_def);
            register_catalog(catalog);
            let library = json!([plain.id, plain.id, plain.id, plain.id]);
            let mut s = scenario(json!({
                "seed": "r5236000-launch",
                "active": "p1",
                "p1": {
                    "hand": [bolt.id, plain.id],
                    "field": [{ "def": booster.id, "lane": 1 }],
                    "library": library,
                },
                "p2": { "hand": [plain.id], "field": [{ "def": plain.id, "lane": 1 }], "library": library },
            }));
            for player in [P1, P2] {
                s.state_mut().players[player].auto_end_turn = Some(false);
            }
            let piece = s.unit_at(P1, 1).id;
            s.play(bolt.id.as_str(), pick::targets(pick::id(&piece)));
            let ship = dormant_starship_of(s.state(), P1)
                .cloned()
                .unwrap_or_else(|| panic!("setup: the dead piece left p1 no Dormant Starship"));
            (s, ship)
        }

        #[test]
        fn r5236000_a_starship_launch_spends_none() {
            let (mut s, ship) = built_ship();
            assert_eq!(
                durability_left(s.state(), &ship),
                Some(SHIP_DURABILITY),
                "setup: the hull prints Durability"
            );
            s.activate(ship.id.as_str(), json!({ "ability": STARSHIP_LAUNCH_ABILITY }));
            assert_eq!(durability_counts(s.last_events(), &ship.id), 0);
            assert_eq!(s.card(ship.id.as_str()).durability_spent.unwrap_or(0), 0);
        }
    }
}

mod r5236001_at_0_its_activates_are_refused {
    use super::*;

    /// MANY with both of its Durability spent.
    fn spent(seed: &str) -> (Scenario, String) {
        let mut s = board(seed, json!({ "field": [MANY] }), json!({}));
        let id = s.unit_at(P1, 1).id;
        for _ in 0..MANY_DURABILITY {
            s.activate(id.as_str(), json!({}));
        }
        assert_eq!(left(&s, &id), Some(0), "setup: every use spent");
        (s, id)
    }

    #[test]
    fn r5236001_at_0_the_activate_is_refused_and_legal_actions_offer_none() {
        let (mut s, id) = spent("r5236001-refused");
        let before = mana(&s, P1);
        s.expect_refused_with(|s| s.activate(id.as_str(), json!({})), NO_DURABILITY);
        assert_eq!(mana(&s, P1), before, "the refused Activate did nothing");
        assert_eq!(left(&s, &id), Some(0), "a refusal spends nothing and never goes below 0");
        assert!(s.activations_of(id.as_str()).is_empty(), "{:?}", s.activations_of(id.as_str()));
        let why = refusal(&s, &id, None);
        assert!(
            why.as_deref().is_some_and(|text| text.contains(NO_DURABILITY)),
            "{why:?}"
        );
    }

    #[test]
    fn r5236001_the_view_shows_the_count_left_and_at_0_an_unusable_ability() {
        let mut s = board("r5236001-view", json!({ "field": [MANY] }), json!({}));
        let id = s.unit_at(P1, 1).id;
        let fresh = views(&s, &id);
        assert_eq!(fresh.len(), 1, "{fresh:?}");
        assert_eq!(fresh[0].durability_left, Some(MANY_DURABILITY));
        assert!(fresh[0].usable);
        for _ in 0..MANY_DURABILITY {
            s.activate(id.as_str(), json!({}));
        }
        let spent = views(&s, &id);
        assert_eq!(spent.len(), 1, "{spent:?}");
        assert_eq!(spent[0].durability_left, Some(0));
        assert!(!spent[0].usable, "{spent:?}");
        assert!(spent[0].reason.is_some(), "an unusable ability says why: {spent:?}");
    }

    #[test]
    fn r5236001_a_unit_at_0_stays_on_the_field_and_is_still_refused_on_its_next_turn() {
        let (mut s, id) = spent("r5236001-stays");
        s.expect_in_zone(id.as_str(), "field");
        s.end_turn();
        s.end_turn();
        assert_eq!(s.state().active, P1);
        s.expect_in_zone(id.as_str(), "field");
        assert_eq!(left(&s, &id), Some(0), "the count is the card's, not the turn's");
        s.expect_refused_with(|s| s.activate(id.as_str(), json!({})), NO_DURABILITY);
        assert!(s.activations_of(id.as_str()).is_empty());
    }
}

mod r5236002_a_field_spell_at_0_goes_to_the_graveyard {
    use super::*;

    fn field_board(seed: &str, def: &str) -> (Scenario, String) {
        let s = board(
            seed,
            json!({ "backrow": [{ "def": def, "faceUp": true }] }),
            json!({}),
        );
        let id = backrow_id(&s, P1, 1);
        (s, id)
    }

    #[test]
    fn r5236002_after_its_activation_resolves_it_goes_to_its_owners_graveyard_and_no_death_runs() {
        let (mut s, id) = field_board("r5236002-field", FIELD);
        let before = mana(&s, P1);
        let health = hero_health(&s, P2);
        s.activate(id.as_str(), json!({}));
        assert_eq!(mana(&s, P1), before + GAIN, "the activation resolved");
        s.expect_in_zone(id.as_str(), "graveyard");
        assert!(s.pile(P1, "graveyard").iter().any(|card| card.id == id));
        assert!(s.backrow(P1, 1).is_none());
        assert!(!destroyed(s.last_events(), &id), "a plain move, no death");
        assert_eq!(hero_health(&s, P2), health, "its Death never ran");
        assert_eq!(s.state().counters.destroyed, 0);
    }

    #[test]
    fn r5236002_indestructible_does_not_keep_it_on_the_field() {
        let (mut s, id) = field_board("r5236002-warded", WARDED);
        let health = hero_health(&s, P2);
        s.activate(id.as_str(), json!({}));
        s.expect_in_zone(id.as_str(), "graveyard");
        assert!(!destroyed(s.last_events(), &id));
        assert_eq!(hero_health(&s, P2), health, "its Death never ran");
    }

    #[test]
    fn r5236002_a_field_spell_with_durability_left_stays_on_the_field() {
        let (mut s, id) = field_board("r5236002-lasting", LASTING);
        s.activate(id.as_str(), json!({}));
        s.expect_in_zone(id.as_str(), "field");
        assert_eq!(left(&s, &id), Some(LASTING_DURABILITY - 1));
        assert!(!s.pile(P1, "graveyard").iter().any(|card| card.id == id));
    }
}

mod r5236003_refresh_and_a_counter_never_give_it_back {
    use super::*;

    #[test]
    fn r5236003_refresh_gives_back_the_use_but_never_the_durability() {
        let mut s = board("r5236003-refresh", json!({ "field": [ONCE] }), json!({}));
        let id = s.unit_at(P1, 1).id;
        s.activate(id.as_str(), json!({}));
        assert_eq!(left(&s, &id), Some(ONCE_DURABILITY - 1));
        apply_now(s.state_mut(), refresh_activates());
        assert_eq!(
            ability_uses_this_turn(s.state(), s.card(id.as_str()), GAIN_ABILITY),
            0,
            "the use is back"
        );
        assert_eq!(left(&s, &id), Some(ONCE_DURABILITY - 1), "the Durability is not");
        s.activate(id.as_str(), json!({}));
        assert_eq!(left(&s, &id), Some(0));
        apply_now(s.state_mut(), refresh_activates());
        assert_eq!(left(&s, &id), Some(0));
        s.expect_refused_with(|s| s.activate(id.as_str(), json!({})), NO_DURABILITY);
    }

    #[test]
    fn r5236003_a_countered_activate_keeps_its_durability_spent() {
        let mut s = board(
            "r5236003-countered",
            json!({ "backrow": [COUNTER] }),
            json!({ "field": [ONCE] }),
        );
        s.end_turn();
        assert_eq!(s.state().active, P2);
        let id = s.unit_at(P2, 1).id;
        let before = mana(&s, P2);
        s.activate(id.as_str(), json!({ "ability": GAIN_ABILITY }));
        assert!(
            s.last_events()
                .iter()
                .any(|event| event.event_type() == GameEventType::TrapFired),
            "setup: the counter answered"
        );
        assert_eq!(mana(&s, P2), before, "the countered Activate did nothing");
        assert_eq!(
            ability_uses_this_turn(s.state(), s.card(id.as_str()), GAIN_ABILITY),
            1
        );
        assert_eq!(left(&s, &id), Some(ONCE_DURABILITY - 1));
    }
}

mod r5236004_leaving_the_field_resets_it {
    use super::*;

    #[test]
    fn r5236004_a_bounced_card_comes_back_with_its_durability_full() {
        let mut s = board("r5236004-bounce", json!({ "field": [ONCE] }), json!({}));
        let id = s.unit_at(P1, 1).id;
        s.activate(id.as_str(), json!({}));
        assert_eq!(left(&s, &id), Some(ONCE_DURABILITY - 1));
        apply_now(
            s.state_mut(),
            bounce(json_as(json!({ "target": { "of": "instance", "instanceId": id } }))),
        );
        s.expect_in_zone(id.as_str(), "hand");
        assert_eq!(s.card(id.as_str()).durability_spent, None, "the reset clears it");
        s.play(id.as_str(), json!({ "zone": 1 }));
        let back = s.unit_at(P1, 1).id;
        assert_eq!(left(&s, &back), Some(ONCE_DURABILITY));
    }

    #[test]
    fn r5236004_the_count_does_not_reset_while_the_card_stays_on_the_field() {
        let mut s = board("r5236004-stays", json!({ "field": [ONCE] }), json!({}));
        let id = s.unit_at(P1, 1).id;
        s.activate(id.as_str(), json!({}));
        s.end_turn();
        s.end_turn();
        assert_eq!(s.state().active, P1);
        assert_eq!(left(&s, &id), Some(ONCE_DURABILITY - 1));
        assert_eq!(s.card(id.as_str()).durability_spent, Some(1));
    }
}

mod r5236005_buff_and_nerf {
    use super::*;

    #[test]
    fn r5236005_a_buff_raises_durability_by_1() {
        let s = board("r5236005-buff", json!({ "hand": [ONCE, PLAIN] }), json!({}));
        let id = hand_id(&s, ONCE);
        assert_eq!(left(&s, &id), Some(ONCE_DURABILITY));
        let after = after_one(&s, &id, TuneDirection::Upgrade);
        assert!(after.contains(&Some(ONCE_DURABILITY + 1)), "{after:?}");
        assert!(
            after
                .iter()
                .all(|n| n.is_none_or(|n| n == ONCE_DURABILITY || n == ONCE_DURABILITY + 1)),
            "one Buff moves it by 1 at most, and only up: {after:?}"
        );
    }

    #[test]
    fn r5236005_a_nerf_lowers_durability_by_1() {
        let s = board("r5236005-nerf", json!({ "hand": [SHARED, PLAIN] }), json!({}));
        let id = hand_id(&s, SHARED);
        let after = after_one(&s, &id, TuneDirection::Degrade);
        assert!(after.contains(&Some(SHARED_DURABILITY - 1)), "{after:?}");
        assert!(
            after
                .iter()
                .all(|n| n.is_none_or(|n| n == SHARED_DURABILITY || n == SHARED_DURABILITY - 1)),
            "one Nerf moves it by 1 at most, and only down: {after:?}"
        );
    }

    #[test]
    fn r5236005_a_nerf_never_takes_durability_below_1() {
        let s = board("r5236005-floor", json!({ "hand": [SINGLE, PLAIN] }), json!({}));
        let id = hand_id(&s, SINGLE);
        let after = after_one(&s, &id, TuneDirection::Degrade);
        assert!(
            after.iter().all(|n| n.is_none_or(|n| n == SINGLE_DURABILITY)),
            "Durability 1 is its floor: {after:?}"
        );
        let mut state = s.state().clone();
        for _ in 0..NERFS_IN_A_ROW {
            tuned_once(&mut state, &id, TuneDirection::Degrade);
            let now = durability_left(&state, &live(&state, &id));
            assert!(now.is_none_or(|n| n >= SINGLE_DURABILITY), "{now:?}");
        }
    }
}

mod r5236006_core_79_twinspell {
    use super::*;

    const TWINSPELL: &str = "core-079";
    /// The Coin (§7): "Gain 1 mana this turn.", so each resolution of it is a mana.
    const COIN: &str = "core-t-coin";
    /// Mr. Vanilla, a (1) Unit.
    const VANILLA: &str = "core-008";
    const P2_HAND: &str = "core-005";
    const P2_FIELD: &str = "core-019";
    const TWINSPELL_COST: i32 = 1;
    /// Twinspell's base face's Durability (FV0: "an exact Durability 1").
    const TWINSPELL_DURABILITY: i32 = 1;

    fn real_twinspell() -> CardDef {
        let catalog: CardDefs = serde_json::from_str(include_str!("../../../cards/catalog.json"))
            .expect("crates/cards/catalog.json parses as CardDefs");
        catalog
            .get(TWINSPELL)
            .cloned()
            .unwrap_or_else(|| panic!("{TWINSPELL} is in the catalog"))
    }

    fn durabilities(keywords: &[Keyword]) -> Vec<i32> {
        keywords
            .iter()
            .filter_map(|keyword| match keyword {
                Keyword::Durability { n } => Some(*n),
                _ => None,
            })
            .collect()
    }

    /// The real cards: p1 holds Twinspell (Radiant or not), The Coin and a Vanilla at `MAX_MANA`.
    fn with_coin(seed: &str, radiant: bool) -> Scenario {
        jackioh_cards::register_all();
        let library = json!([VANILLA, VANILLA, VANILLA, VANILLA]);
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": TWINSPELL, "radiant": radiant }, COIN, VANILLA],
                "mana": MAX_MANA,
                "library": library,
            },
            "p2": { "hand": [P2_HAND], "field": [P2_FIELD], "library": library },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    #[test]
    fn r5236006_its_base_face_prints_durability_1_and_its_radiant_face_none() {
        let def = real_twinspell();
        assert_eq!(durabilities(&def.base.keywords), vec![TWINSPELL_DURABILITY]);
        assert_eq!(durabilities(&def.radiant.keywords), Vec::<i32>::new());
    }

    #[test]
    fn r5236006_unactivated_the_base_face_grants_nothing_and_stays_with_its_durability() {
        let mut s = with_coin("r5236006-unused", false);
        s.play(TWINSPELL, json!({}));
        assert_eq!(left(&s, TWINSPELL), Some(TWINSPELL_DURABILITY));
        s.play(COIN, json!({}));
        // One resolution of The Coin: no Echo was granted.
        s.expect_mana(P1, MAX_MANA - TWINSPELL_COST + 1);
        s.expect_in_zone(TWINSPELL, "field");
    }

    #[test]
    fn r5236006_activated_the_base_face_grants_echo_1_and_goes_to_the_graveyard_by_a_plain_move() {
        let mut s = with_coin("r5236006-used", false);
        let twin = s.card(TWINSPELL).clone();
        s.play(TWINSPELL, json!({}));
        s.activate(TWINSPELL, json!({}));
        s.expect_in_zone(&twin, "graveyard");
        assert!(!destroyed(s.last_events(), &twin.id), "no death");
        assert_eq!(s.state().counters.destroyed, 0);
        // The next Spell resolves one more time: two Coins' worth of mana.
        s.play(COIN, json!({}));
        s.expect_mana(P1, MAX_MANA - TWINSPELL_COST + 2);
    }

    #[test]
    fn r5236006_indestructible_does_not_keep_the_base_face_on_the_field() {
        let mut s = with_coin("r5236006-warded", false);
        let twin = s.card(TWINSPELL).clone();
        s.play(TWINSPELL, json!({}));
        s.card_mut(TWINSPELL)
            .granted_keywords
            .push(Keyword::Indestructible);
        s.activate(TWINSPELL, json!({}));
        s.expect_in_zone(&twin, "graveyard");
        assert!(!destroyed(s.last_events(), &twin.id));
    }

    #[test]
    fn r5236006_the_radiant_face_stays_and_serves_again_next_turn() {
        let mut s = with_coin("r5236006-radiant", true);
        let twin = s.card(TWINSPELL).clone();
        s.play(TWINSPELL, json!({}));
        assert_eq!(left(&s, &twin.id), None, "the Radiant face prints no Durability");
        s.activate(TWINSPELL, json!({}));
        s.expect_in_zone(&twin, "field");
        s.end_turn();
        s.end_turn();
        assert_eq!(s.state().active, P1);
        assert_eq!(refusal(&s, &twin.id, None), None);
        s.activate(TWINSPELL, json!({}));
        s.expect_in_zone(&twin, "field");
    }
}
