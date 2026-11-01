//! Every Activate ability, used the way a client uses it (#491, SPEC §6.2, R384, R752): the
//! `activate` is found in `legal_actions`, the controller's own view lists the ability as usable,
//! and the very action listed goes through `reduce` and does what the card's text says, on the base
//! face and on the Radiant one.
//! The per-card tests mostly drive an ability through the testkit's `activate` verb, which builds its
//! own action (the `activatePower` alias when it names no ability, R752). A client never builds one:
//! it sends back an action `legal_actions` listed, and online that action is sent over the socket
//! (`crates/server/tests/actor/match_actor.rs`, `r384_activate_over_the_socket`). This file is the
//! engine half of that path for every Activate card at once, and `activate_census` keeps it whole:
//! a card whose text gains an Activate must gain a case here.

use jackioh_engine::config::LIFE_TAP_DAMAGE;
use jackioh_engine::testkit::*;

use super::{scenario, targets_of, tributes_of};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

const HEROIC_POWER: &str = "core-098";
const INFINISCEPTER: &str = "classic-007";
const NOSE_HUNTER: &str = "classic-015";
const POWER_TO_PUNISH: &str = "classic-020";
const TURTINATOR: &str = "classic-021";
const DEVILS_PACT: &str = "classic-023";
const TRANSMUTABLE_TOXINS: &str = "classic-042";
const MALZAHARS_RECYCLER: &str = "classic-064";
const CORPSE_PLANTATION: &str = "classic-074";
const MUTATE_SPELL: &str = "classic-078";
const POWER_TO_THRIVE: &str = "classic-081";
const LOCKDOWN: &str = "classic-084";
const THE_HOUSE: &str = "classicplus-007";
const FUSION_LAB: &str = "classicplus-031";
const DOCTORS_ORDERS: &str = "classicplus-060";
const KYS_GIFT: &str = "classicplus-042-1";
const CLAUDES_DATACENTER: &str = "classicplus-078";
const BROTHER_PING: &str = "classicplus-076-1";
const WISHING_WELL: &str = "meditative-097-2";
const SCHOOL: &str = "meditative-097-3";
const MEGA_CHURCH: &str = "meditative-097-4";
const UNIVERSITY: &str = "meditative-097-6";
const HEADQUARTERS: &str = "meditative-097-9";
const MONEY_MACHINE: &str = "meditative-054";
const GACHAHOLIC: &str = "meditative-101";
const ALUNETH: &str = "meditative-080";
const YILES_VIRUS: &str = "meditative-049-2";
const MAESTRO: &str = "meditative-069";
const EVER_GROWING_TREE: &str = "meditative-075";
const LIVING_MANA_POTION: &str = "meditativeplus-011";
const LIVING_ARMOR_POTION: &str = "meditativeplus-012";
const LIVING_HEALTH_POTION: &str = "meditativeplus-013";

const ACTIVATE_CARDS: [&str; 71] = [
    HEROIC_POWER,
    INFINISCEPTER,
    NOSE_HUNTER,
    POWER_TO_PUNISH,
    TURTINATOR,
    DEVILS_PACT,
    TRANSMUTABLE_TOXINS,
    MALZAHARS_RECYCLER,
    CORPSE_PLANTATION,
    MUTATE_SPELL,
    POWER_TO_THRIVE,
    LOCKDOWN,
    THE_HOUSE,
    FUSION_LAB,
    DOCTORS_ORDERS,
    KYS_GIFT,
    CLAUDES_DATACENTER,
    BROTHER_PING,
    WISHING_WELL,
    SCHOOL,
    MEGA_CHURCH,
    UNIVERSITY,
    HEADQUARTERS,
    MONEY_MACHINE,
    GACHAHOLIC,
    ALUNETH,
    YILES_VIRUS,
    MAESTRO,
    EVER_GROWING_TREE,
    LIVING_MANA_POTION,
    LIVING_ARMOR_POTION,
    LIVING_HEALTH_POTION,
    "stinkyscape-018-1",
    "stinkyscape-018-2",
    "stinkyscape-018-3",
    "stinkyscape-026",
    "stinkyscape-039",
    "stinkyscape-050",
    "stinkyscape-051",
    "stinkyscape-052",
    "stinkyscape-053",
    "stinkyscape-054",
    "stinkyscape-055",
    "stinkyscape-056",
    "stinkyscape-057",
    "stinkyscape-058",
    "stinkyscape-059",
    "stinkyscape-060",
    "stinkyscape-061",
    "stinkyscape-062",
    "stinkyscape-063",
    "stinkyscape-064",
    "stinkyscape-065",
    "stinkyscape-066",
    "stinkyscape-112",
    "stinkyscape-116",
    "stinkyscape-125",
    "stinkyscape-140",
    "stinkyscape-141",
    "stinkyscape-142",
    "stinkyscape-143",
    "stinkyscape-148",
    "stinkyscape-157",
    "stinkyscape-169",
    "stinkyscape-170",
    "stinkyscape-171",
    "stinkyscape-172",
    "stinkyscape-173",
    "stinkyscape-175",
    "stinkyscape-t-dormant-starship",
    "stinkyscape-t-propagation-layer",
];

const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2. A spare that keeps R82's auto-end away.
const MANA_WELL: &str = "core-006"; // (3) Field Spell: a backrow card of p1's to hold a Plague Counter.
const VANILLA: &str = "core-008"; // (1) Unit 4/4.
const FILLER: &str = "core-010"; // (0) Spell.
const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
const X: &str = "core-020"; // library filler.
const CONJURE_KY: &str = "core-057"; // (2) Spell: Add 3 random KY cards to your hand.
const APPLE: &str = "classicplus-059"; // All Purpose Apple, Doctors Orders' card.
const THE_COIN: &str = "core-t-coin"; // Money Machine's card.

fn on(def: &str, radiant: bool) -> Value {
    json!({ "def": def, "radiant": radiant })
}

fn hero(player: PlayerId) -> Selection {
    Selection::Hero { player }
}

fn instance(id: &str) -> Selection {
    Selection::Instance {
        instance_id: id.to_string(),
    }
}

fn listed(s: &Scenario, card: &str) -> Vec<ActionBody> {
    legal_actions(s.state(), P1)
        .into_iter()
        .filter(|action| matches!(action, ActionBody::Activate { instance_id, .. } if instance_id == card))
        .collect()
}

fn modes_of(action: &ActionBody) -> Vec<String> {
    match action {
        ActionBody::Activate { modes, .. } => modes.clone().unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn ability_of(action: &ActionBody) -> Option<String> {
    match action {
        ActionBody::Activate { ability, .. } => ability.clone(),
        _ => None,
    }
}

fn activations_in(value: &Value, card: &str) -> Option<Vec<Value>> {
    match value {
        Value::Object(fields) => {
            if fields.get("instanceId") == Some(&json!(card))
                && let Some(Value::Array(list)) = fields.get("activations")
            {
                return Some(list.clone());
            }
            fields.values().find_map(|inner| activations_in(inner, card))
        }
        Value::Array(items) => items.iter().find_map(|inner| activations_in(inner, card)),
        _ => None,
    }
}

/// R384, §10.8: the abilities p1's own view lists on the card.
fn viewed(s: &Scenario, card: &str) -> Vec<Value> {
    let view = serde_json::to_value(s.view(P1)).expect("PlayerView serializes");
    activations_in(&view["you"], card).unwrap_or_default()
}

fn activate_listed(s: &mut Scenario, card: &str, pick: impl Fn(&ActionBody) -> bool) -> Vec<GameEvent> {
    let id = s.card(card).id.clone();
    let offered = listed(s, &id);
    assert!(
        !offered.is_empty(),
        "legal_actions lists no `activate` for {card}"
    );
    let Some(action) = offered.into_iter().find(|action| pick(action)) else {
        panic!("legal_actions lists no `activate` of {card} that the test asks for")
    };
    let ability = ability_of(&action).expect("a listed activation names its ability");
    assert!(
        viewed(s, &id)
            .iter()
            .any(|view| view["ability"] == json!(ability) && view["usable"] == json!(true)),
        "p1's view does not show {card}'s {ability} as usable: {:?}",
        viewed(s, &id)
    );

    let nonce = format!("listed-{}-{}", s.state().turn, s.state().applied.len());
    let result = reduce(s.state(), &Action::new(action, P1, nonce));
    assert_eq!(
        result.error, None,
        "reduce refused the `activate` legal_actions listed for {card}"
    );
    assert!(
        result.events.iter().any(
            |event| matches!(event, GameEvent::Activated { instance_id, ability: used, .. } if *instance_id == id && *used == ability)
        ),
        "no `activated` event for {card}"
    );
    *s.state_mut() = result.state;
    result.events
}

fn any(_: &ActionBody) -> bool {
    true
}

fn health(s: &Scenario, player: PlayerId) -> i32 {
    s.state().players[player].hero.health
}

fn units(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
    (1..=5).filter_map(|lane| s.unit(player, lane)).collect()
}

/// (R1852003) and Alteran Reach's travel (`subsystems::starscape_abilities`, R1847001).
const ENGINE_ACTIVATES: [&str; 2] = ["stinkyscape-t-dormant-starship", "stinkyscape-148"];

/// release, R1857000).
#[test]
fn r384_activate_census_every_card_whose_text_has_an_activate_declares_one_and_has_a_case_here() {
    jackioh_cards::register_all();
    let scripts = jackioh_cards::scripts_of();
    let mut by_text: Vec<String> = Vec::new();
    let mut by_script: Vec<String> = Vec::new();
    for (id, def) in jackioh_cards::CATALOG.iter() {
        // R1857000: held until the release (#1884)
        if !set_ships(def.set) {
            continue;
        }
        if def.base.text.contains("Activate") || def.radiant.text.contains("Activate") {
            by_text.push(id.clone());
        }
        if ENGINE_ACTIVATES.contains(&id.as_str())
            || scripts.get(id).is_some_and(|script| {
                !script.base.activations.is_empty() || !script.radiant.activations.is_empty()
            })
        {
            by_script.push(id.clone());
        }
    }
    let mut covered: Vec<String> = ACTIVATE_CARDS.iter().map(|id| id.to_string()).collect();
    by_text.sort();
    by_script.sort();
    covered.sort();
    assert_eq!(
        by_text, covered,
        "a card's text gained or lost an Activate: give it a case here"
    );
    assert_eq!(
        by_script, covered,
        "a script's Activate does not match its card's text"
    );
}

mod c_n98_heroic_power {
    use super::*;

    /// A Heroic Power on p1's backrow with the named power (R103's stored name, as `ensure_power`
    fn with_power(radiant: bool, power: &str) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "backrow": [on(HEROIC_POWER, radiant)], "hand": [STOCKPILE], "library": [X, X, X], "mana": 3 },
            "p2": { "field": [VANILLA], "hand": [FILLER], "library": [X, X, X] },
        }));
        let id = s.card(HEROIC_POWER).id.clone();
        let live = find_instance_mut(s.state_mut(), &id).expect("the Heroic Power is in the state");
        live.memory
            .insert(subsystems::POWER_KEY.to_string(), json!(power));
        s
    }

    #[test]
    fn r752_r753_life_tap_listed_paid_and_used_base_draw_1_and_take_2() {
        let mut s = with_power(false, "draw");
        let (hand, life) = (s.hand(P1).len(), health(&s, P1));
        activate_listed(&mut s, HEROIC_POWER, any);
        assert_eq!(s.hand(P1).len(), hand + 1);
        assert_eq!(health(&s, P1), life - LIFE_TAP_DAMAGE);
        s.expect_mana(P1, 2);
        let id = s.card(HEROIC_POWER).id.clone();
        assert!(listed(&s, &id).is_empty(), "a power is once per turn");
    }

    /// R753: the top card of each player's deck, both into your hand, and no damage.
    #[test]
    fn r752_r753_life_tap_radiant_draws_1_from_each_players_deck() {
        let mut s = with_power(true, "draw");
        let (hand, theirs, life) = (s.hand(P1).len(), s.pile(P2, "library").len(), health(&s, P1));
        activate_listed(&mut s, HEROIC_POWER, any);
        assert_eq!(s.hand(P1).len(), hand + 2);
        assert_eq!(s.pile(P2, "library").len(), theirs - 1);
        assert_eq!(health(&s, P1), life);
        s.expect_mana(P1, 2);
    }

    #[test]
    fn r756_ping_is_listed_once_per_target_and_the_one_sent_back_hits_it() {
        let mut s = with_power(false, "ping");
        let id = s.card(HEROIC_POWER).id.clone();
        let vanilla = s.card(VANILLA).id.clone();
        let offered: Vec<Vec<Selection>> = listed(&s, &id).iter().map(targets_of).collect();
        assert!(offered.contains(&vec![hero(P2)]));
        assert!(offered.contains(&vec![instance(&vanilla)]));
        activate_listed(&mut s, HEROIC_POWER, |action| {
            targets_of(action) == vec![hero(P2)]
        });
        s.expect_health(P2, 29);
    }
}

mod c_n7_infiniscepter {
    use super::*;

    fn scepter_holding(radiant: bool, spell: &str) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [on(INFINISCEPTER, radiant), spell, FILLER], "library": [X, X, X, X], "health": 20 },
            "p2": { "hand": [FILLER], "library": [X, X] },
        }));
        let picked = s.card(spell).id.clone();
        s.play(
            INFINISCEPTER,
            json!({ "zone": 1, "targets": [{ "pick": "instance", "instanceId": picked }] }),
        );
        s.expect_in_zone(&picked, "exile");
        s
    }

    #[test]
    fn r384_base_casts_a_copy_of_the_1_cost_spell_it_exiled() {
        let mut s = scepter_holding(false, STOCKPILE);
        let hand = s.hand(P1).len();
        activate_listed(&mut s, INFINISCEPTER, any);
        assert_eq!(s.hand(P1).len(), hand + 2);
        s.expect_health(P1, 22);
        let id = s.card(INFINISCEPTER).id.clone();
        assert!(listed(&s, &id).is_empty(), "once per turn");
    }

    #[test]
    fn r384_radiant_casts_a_copy_of_the_2_cost_spell_it_exiled() {
        let mut s = scepter_holding(true, CONJURE_KY);
        let hand = s.hand(P1).len();
        activate_listed(&mut s, INFINISCEPTER, any);
        assert_eq!(s.hand(P1).len(), hand + 3, "Conjure KY's copy adds 3 KY cards");
    }
}

mod c_n15_nose_hunter {
    use super::*;

    fn hunter(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "field": [on(NOSE_HUNTER, radiant)], "hand": [STOCKPILE, STOCKPILE, STOCKPILE] },
            "p2": { "hand": [FILLER, FILLER], "library": [X, X, X] },
        }))
    }

    #[test]
    fn r384_base_discards_a_random_card_and_exiles_the_bottom_of_their_deck_as_often_as_it_pays() {
        let mut s = hunter(false);
        activate_listed(&mut s, NOSE_HUNTER, any);
        assert_eq!(s.hand(P1).len(), 2);
        assert_eq!(s.pile(P2, "library").len(), 2);
        assert_eq!(s.hand(P2).len(), 2);
        activate_listed(&mut s, NOSE_HUNTER, any);
        assert_eq!(s.pile(P2, "library").len(), 1);
    }

    #[test]
    fn r384_radiant_also_exiles_a_random_card_from_their_hand() {
        let mut s = hunter(true);
        activate_listed(&mut s, NOSE_HUNTER, any);
        assert_eq!(s.hand(P1).len(), 2);
        assert_eq!(s.pile(P2, "library").len(), 2);
        assert_eq!(s.hand(P2).len(), 1);
    }
}

mod c_n20_the_power_to_punish {
    use super::*;

    const DAMAGE_MODE: &str = "deal damage";

    fn punish(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "backrow": [on(POWER_TO_PUNISH, radiant)], "hand": [STOCKPILE] },
            "p2": { "field": [VANILLA], "hand": [FILLER, FILLER] },
        }))
    }

    fn listed_modes(s: &Scenario) -> Vec<String> {
        let id = s.card(POWER_TO_PUNISH).id.clone();
        let mut modes: Vec<String> = listed(s, &id).iter().flat_map(modes_of).collect();
        modes.sort();
        modes.dedup();
        modes
    }

    fn at_their_hero(action: &ActionBody) -> bool {
        modes_of(action) == vec![DAMAGE_MODE.to_string()] && targets_of(action) == vec![hero(P2)]
    }

    #[test]
    fn r384_base_lists_its_three_modes_and_deals_2() {
        let mut s = punish(false);
        assert_eq!(
            listed_modes(&s),
            vec![
                DAMAGE_MODE.to_string(),
                "destroy a Unit at the start of your next turn".to_string(),
                "opponent discards".to_string(),
            ]
        );
        activate_listed(&mut s, POWER_TO_PUNISH, at_their_hero);
        s.expect_health(P2, 28);
        let id = s.card(POWER_TO_PUNISH).id.clone();
        assert!(listed(&s, &id).is_empty(), "once per turn");
    }

    #[test]
    fn r384_radiant_lists_its_destroy_all_and_deals_4() {
        let mut s = punish(true);
        assert_eq!(
            listed_modes(&s),
            vec![
                DAMAGE_MODE.to_string(),
                "destroy all enemy Units at the start of your next turn".to_string(),
                "opponent discards".to_string(),
            ]
        );
        activate_listed(&mut s, POWER_TO_PUNISH, at_their_hero);
        s.expect_health(P2, 26);
    }

    #[test]
    fn r384_radiant_the_opponent_discards_2() {
        let mut s = punish(true);
        activate_listed(&mut s, POWER_TO_PUNISH, |action| {
            modes_of(action) == vec!["opponent discards".to_string()]
        });
        assert_eq!(s.hand(P2).len(), 0);
    }
}

mod c_n21_turtinator {
    use super::*;

    fn turtle(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "field": [on(TURTINATOR, radiant), VANILLA], "hand": [STOCKPILE] },
            "p2": { "hand": [FILLER] },
        }))
    }

    fn eats_vanilla_at_their_hero(s: &Scenario) -> impl Fn(&ActionBody) -> bool + use<> {
        let vanilla = s.card(VANILLA).id.clone();
        move |action: &ActionBody| {
            tributes_of(action) == vec![vanilla.clone()] && targets_of(action) == vec![hero(P2)]
        }
    }

    #[test]
    fn r683_it_never_lists_itself_as_the_tribute() {
        let s = turtle(false);
        let id = s.card(TURTINATOR).id.clone();
        assert!(
            listed(&s, &id)
                .iter()
                .all(|action| !tributes_of(action).contains(&id))
        );
    }

    #[test]
    fn r384_base_tributes_the_listed_unit_and_deals_its_attack() {
        let mut s = turtle(false);
        let pick = eats_vanilla_at_their_hero(&s);
        activate_listed(&mut s, TURTINATOR, pick);
        s.expect_health(P2, 26).expect_in_zone(VANILLA, "graveyard");
    }

    #[test]
    fn r384_radiant_deals_twice_its_attack() {
        let mut s = turtle(true);
        let pick = eats_vanilla_at_their_hero(&s);
        activate_listed(&mut s, TURTINATOR, pick);
        s.expect_health(P2, 22);
    }
}

mod c_n23_devils_pact {
    use super::*;

    fn pact(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "backrow": [{ "def": DEVILS_PACT, "radiant": radiant, "lane": 1 }], "hand": [VANILLA, STOCKPILE, FILLER], "library": [X, X] },
            "p2": { "hand": [FILLER], "library": [X, X] },
        }))
    }

    fn play_vanilla_as_a_book(s: &mut Scenario) {
        s.play(VANILLA, json!({ "zone": 2 }));
        s.answer(json!([{ "pick": "hero", "player": "p2" }]));
        assert!(
            units(s, P1).is_empty(),
            "Mr. Vanilla was replaced and took no zone"
        );
    }

    #[test]
    fn r449_base_each_card_played_this_turn_is_a_book_of_flame() {
        let mut s = pact(false);
        activate_listed(&mut s, DEVILS_PACT, any);
        play_vanilla_as_a_book(&mut s);
        s.expect_health(P2, 25); // 4, raised by its Spell Damage +1 (R1563002).
    }

    #[test]
    fn r449_radiant_each_card_played_this_turn_is_a_base_book_of_flame() {
        let mut s = pact(true);
        activate_listed(&mut s, DEVILS_PACT, any);
        play_vanilla_as_a_book(&mut s);
        // A base Book since #2179 (a Radiant one's 8 before): 4, raised by its Spell Damage +2 (R1563002).
        s.expect_health(P2, 24);
    }
}

mod c_n42_transmutable_toxins {
    use super::*;

    fn toxins(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "backrow": [on(TRANSMUTABLE_TOXINS, radiant)], "field": [VANILLA], "hand": [STOCKPILE] },
            "p2": { "hand": [FILLER] },
        }))
    }

    fn grows_by(radiant: bool, per: i32) {
        let mut s = toxins(radiant);
        activate_listed(&mut s, TRANSMUTABLE_TOXINS, any);
        let tokens = s.card(VANILLA).counters.plague.unwrap_or(0);
        assert!(tokens >= 1, "no Plague Counter was placed");
        s.expect_stats(VANILLA, json!({ "attack": 4 + per * tokens }));
        let id = s.card(TRANSMUTABLE_TOXINS).id.clone();
        assert!(listed(&s, &id).is_empty(), "once per turn");
    }

    #[test]
    fn r384_base_places_plague_counters_your_unit_gains_1_1_each() {
        grows_by(false, 1);
    }

    #[test]
    fn r384_radiant_places_plague_counters_your_unit_gains_2_2_each() {
        grows_by(true, 2);
    }
}

mod c_n64_malzahars_recycler {
    use super::*;

    fn recycler(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "backrow": [on(MALZAHARS_RECYCLER, radiant)], "hand": [STOCKPILE, STOCKPILE, STOCKPILE], "library": [X, X, X, X] },
            "p2": { "hand": [FILLER], "library": [X, X, X] },
        }))
    }

    #[test]
    fn r384_base_discards_2_random_cards_and_draws_2_once_a_turn() {
        let mut s = recycler(false);
        activate_listed(&mut s, MALZAHARS_RECYCLER, any);
        assert_eq!(s.pile(P1, "graveyard").len(), 2);
        assert_eq!(s.hand(P1).len(), 3);
        assert_eq!(s.pile(P1, "library").len(), 2);
        let id = s.card(MALZAHARS_RECYCLER).id.clone();
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }

    #[test]
    fn r384_radiant_discards_2_random_cards_and_draws_the_deck_once_a_turn() {
        let mut s = recycler(true);
        activate_listed(&mut s, MALZAHARS_RECYCLER, any);
        assert_eq!(s.pile(P1, "graveyard").len(), 2);
        assert_eq!(s.hand(P1).len(), 1 + 4);
        assert_eq!(s.pile(P1, "library").len(), 0);
        let id = s.card(MALZAHARS_RECYCLER).id.clone();
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }
}

mod c_n74_corpse_plantation {
    use super::*;

    fn arms(radiant: bool) {
        let mut s = scenario(json!({
            "p1": { "backrow": [on(CORPSE_PLANTATION, radiant)], "hand": [STOCKPILE] },
            "p2": { "hand": [FILLER] },
        }));
        activate_listed(&mut s, CORPSE_PLANTATION, any);
        let turn = s.state().turn;
        assert_eq!(s.card(CORPSE_PLANTATION).memory.get("armed"), Some(&json!(turn)));
        let id = s.card(CORPSE_PLANTATION).id.clone();
        assert!(listed(&s, &id).is_empty(), "once per turn");
    }

    #[test]
    fn r384_base_arms_a_graveyard_play_once_a_turn() {
        arms(false);
    }

    #[test]
    fn r384_radiant_arms_a_graveyard_play_once_a_turn() {
        arms(true);
    }
}

mod c_n78_mutate_spell {
    use super::*;

    fn mutate(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "backrow": [on(MUTATE_SPELL, radiant), { "def": MANA_WELL, "counters": { "plague": 1 } }],
                "hand": [STOCKPILE],
                "library": [X, X, X, X, X, X],
            },
            "p2": { "field": [{ "def": VANILLA, "counters": { "plague": 1 } }], "hand": [FILLER] },
        }))
    }

    fn at(card: String) -> impl Fn(&ActionBody) -> bool {
        move |action: &ActionBody| targets_of(action) == vec![instance(&card)]
    }

    #[test]
    fn r384_base_a_counter_off_your_backrow_card_draws_2_off_an_enemy_unit_exiles_it() {
        let mut s = mutate(false);
        let well = s.card(MANA_WELL).id.clone();
        let enemy = s.card(VANILLA).id.clone();
        let hand = s.hand(P1).len();
        activate_listed(&mut s, MUTATE_SPELL, at(well.clone()));
        assert_eq!(s.hand(P1).len(), hand + 2);
        assert_eq!(s.card(&well).counters.plague.unwrap_or(0), 0);
        let id = s.card(MUTATE_SPELL).id.clone();
        assert!(listed(&s, &id).is_empty(), "once per turn");
        let mut s = mutate(false);
        activate_listed(&mut s, MUTATE_SPELL, at(enemy.clone()));
        s.expect_in_zone(&enemy, "exile");
    }

    #[test]
    fn r384_radiant_a_counter_off_your_backrow_card_draws_4_then_off_an_enemy_unit_exiles_it() {
        let mut s = mutate(true);
        let well = s.card(MANA_WELL).id.clone();
        let enemy = s.card(VANILLA).id.clone();
        let hand = s.hand(P1).len();
        activate_listed(&mut s, MUTATE_SPELL, at(well));
        assert_eq!(s.hand(P1).len(), hand + 4);
        activate_listed(&mut s, MUTATE_SPELL, at(enemy.clone()));
        s.expect_in_zone(&enemy, "exile");
    }
}

mod c_n81_the_power_to_thrive {
    use super::*;

    fn reduces(radiant: bool, discount: i32) {
        let mut s = scenario(json!({
            "p1": { "backrow": [on(POWER_TO_THRIVE, radiant)], "hand": [STOCKPILE], "mana": 3 },
            "p2": { "hand": [FILLER] },
        }));
        activate_listed(&mut s, POWER_TO_THRIVE, |action| {
            modes_of(action) == vec!["reduce".to_string()]
        });
        let stockpile = s.card(STOCKPILE).id.clone();
        s.answer(json!(stockpile));
        assert_eq!(s.card(&stockpile).cost_mod, -discount);
        s.expect_mana(P1, 3);
        let id = s.card(POWER_TO_THRIVE).id.clone();
        assert!(listed(&s, &id).is_empty(), "once per turn");
    }

    #[test]
    fn r384_base_the_reduce_mode_takes_1_off() {
        reduces(false, 1);
    }

    #[test]
    fn r384_radiant_the_reduce_mode_takes_2_off() {
        reduces(true, 2);
    }
}

mod c_n84_lockdown {
    use super::*;

    fn tributes_itself(radiant: bool) {
        let mut s = scenario(json!({
            "p1": { "backrow": [on(LOCKDOWN, radiant)], "hand": [STOCKPILE] },
            "p2": { "hand": [FILLER] },
        }));
        activate_listed(&mut s, LOCKDOWN, any);
        // §6.3: a Tribute is a Sacrifice, which Indestructible does not stop.
        s.expect_in_zone(LOCKDOWN, "graveyard");
    }

    #[test]
    fn r384_base_tributes_itself_past_indestructible() {
        tributes_itself(false);
    }

    #[test]
    fn r384_radiant_tributes_itself_past_indestructible() {
        tributes_itself(true);
    }
}

mod c_plus_n7_the_house {
    use super::*;

    fn house(radiant: bool, counters: i32) -> Scenario {
        scenario(json!({
            "p1": {
                "backrow": [{ "def": THE_HOUSE, "radiant": radiant, "counters": { "plague": counters } }],
                "hand": [STOCKPILE],
            },
            "p2": { "hand": [FILLER] },
        }))
    }

    fn mode(wanted: &'static str) -> impl Fn(&ActionBody) -> bool {
        move |action| modes_of(action) == vec![wanted.to_string()]
    }

    fn twins(s: &Scenario) -> Vec<(String, bool)> {
        units(s, P1)
            .into_iter()
            .map(|unit| (unit.def_id, unit.radiant))
            .collect()
    }

    #[test]
    fn r1704002_base_lists_both_twins_spends_a_counter_and_summons_the_one_named_once_a_turn() {
        let mut s = house(false, 3);
        let id = s.card(THE_HOUSE).id.clone();
        let mut modes: Vec<String> = listed(&s, &id).iter().flat_map(modes_of).collect();
        modes.sort();
        assert_eq!(modes, vec!["attacker", "defender"]);
        activate_listed(&mut s, THE_HOUSE, mode("attacker"));
        assert_eq!(twins(&s), vec![("classicplus-006".to_string(), false)]);
        assert_eq!(s.card(THE_HOUSE).counters.plague, Some(2));
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }

    #[test]
    fn r1704002_radiant_summons_a_radiant_right_house_defender() {
        let mut s = house(true, 3);
        activate_listed(&mut s, THE_HOUSE, mode("defender"));
        assert_eq!(twins(&s), vec![("core-003".to_string(), true)]);
        assert_eq!(s.card(THE_HOUSE).counters.plague, Some(2));
    }

    #[test]
    fn r1704002_it_is_not_listed_with_no_counter() {
        let s = house(false, 0);
        let id = s.card(THE_HOUSE).id.clone();
        assert!(listed(&s, &id).is_empty(), "the Activate spends a Plague Counter");
    }
}

mod c_plus_n78_claudes_datacenter {
    use super::*;

    /// Its controller uses the listed `activate` (an Activate since #2179, R384): free, one AI generated
    fn adds_an_ai_card(radiant: bool) {
        let mut s = scenario(json!({
            "p1": { "backrow": [on(CLAUDES_DATACENTER, radiant)], "hand": [STOCKPILE], "library": [X, X] },
            "p2": { "hand": [FILLER] },
        }));
        let mana = s.state().players.p1.mana.current;
        let before: Vec<String> = s.hand(P1).into_iter().map(|card| card.id).collect();
        activate_listed(&mut s, CLAUDES_DATACENTER, any);
        let added: Vec<CardInstance> = s
            .hand(P1)
            .into_iter()
            .filter(|card| !before.contains(&card.id))
            .collect();
        assert_eq!(added.len(), 1);
        let def = jackioh_cards::card_def(&added[0].def_id);
        assert!(
            def.token && def.tags.contains(&Tag::Ai),
            "{} is no AI generated card",
            added[0].def_id
        );
        assert_eq!(added[0].radiant, radiant);
        assert_eq!(added[0].cost_override, Some(0));
        assert_eq!(s.state().players.p1.mana.current, mana);
        s.expect_in_zone(CLAUDES_DATACENTER, "field");
        let id = s.card(CLAUDES_DATACENTER).id.clone();
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }

    #[test]
    fn r384_base_adds_an_ai_generated_card_costing_0_once_a_turn() {
        adds_an_ai_card(false);
    }

    #[test]
    fn r384_radiant_adds_a_radiant_ai_generated_card_costing_0_once_a_turn() {
        adds_an_ai_card(true);
    }
}

mod c_n31_fusion_lab {
    use super::*;

    fn fuses_into_menace(radiant: bool) -> Vec<FusedIngredient> {
        let mut s = scenario(json!({
            "p1": { "backrow": [on(FUSION_LAB, radiant)], "hand": [MENACE, STOCKPILE] },
            "p2": { "hand": [FILLER] },
        }));
        let menace = s.card(MENACE).id.clone();
        let stockpile = s.card(STOCKPILE).id.clone();
        let lab = s.card(FUSION_LAB).id.clone();
        let offered: Vec<Vec<Selection>> = listed(&s, &lab).iter().map(targets_of).collect();
        assert!(offered.contains(&vec![instance(&menace)]));
        assert!(offered.contains(&vec![instance(&stockpile)]));
        activate_listed(&mut s, FUSION_LAB, |action| {
            targets_of(action) == vec![instance(&menace)]
        });
        let kept = s.card(&menace).clone();
        assert_eq!(kept.zone, Zone::Hand { player: P1 });
        assert_eq!(s.card(&stockpile).def_id, STOCKPILE);
        let specs = fused_id_specs(Some(s.state()), &kept.def_id).expect("the Menace was fused");
        assert_eq!(
            specs.get(1).map(|spec| spec.def_id.clone()),
            Some(MENACE.to_string())
        );
        specs
    }

    #[test]
    fn r384_base_fuses_a_random_card_into_the_chosen_hand_card() {
        let specs = fuses_into_menace(false);
        assert_ne!(specs.first().and_then(|spec| spec.radiant), Some(true));
    }

    #[test]
    fn r384_radiant_fuses_a_random_radiant_card_into_it() {
        let specs = fuses_into_menace(true);
        assert_eq!(specs.first().and_then(|spec| spec.radiant), Some(true));
    }
}

mod c_n60_doctors_orders {
    use super::*;

    fn adds_an_apple(radiant: bool) {
        let mut s = scenario(json!({
            "p1": { "backrow": [on(DOCTORS_ORDERS, radiant)], "hand": [STOCKPILE] },
            "p2": { "hand": [FILLER] },
        }));
        activate_listed(&mut s, DOCTORS_ORDERS, any);
        let apples: Vec<CardInstance> = s
            .hand(P1)
            .into_iter()
            .filter(|card| card.def_id == APPLE)
            .collect();
        assert_eq!(apples.len(), 1);
        assert!(apples.iter().all(|apple| apple.radiant == radiant));
    }

    #[test]
    fn r384_base_adds_an_all_purpose_apple() {
        adds_an_apple(false);
    }

    #[test]
    fn r384_radiant_adds_a_radiant_all_purpose_apple() {
        adds_an_apple(true);
    }
}

mod c_n42_1_kys_gift {
    use super::*;

    /// Patch v0.3.10 (#831): the Gift's start-of-turn effects became its Activate. Used once, it gains
    fn opens(radiant: bool) {
        let mut s = scenario(json!({
            "p1": { "backrow": [on(KYS_GIFT, radiant)], "hand": [STOCKPILE], "library": [X, X] },
            "p2": { "hand": [MENACE, FILLER], "library": [X, X] },
        }));
        let before = s.state().players.p1.mana.current;
        activate_listed(&mut s, KYS_GIFT, any);
        let mana = if radiant { 2 } else { 1 };
        assert_eq!(s.state().players.p1.mana.current, before + mana);
        let added: Vec<CardInstance> = s
            .hand(P1)
            .into_iter()
            .filter(|card| card.cost_override == Some(0))
            .collect();
        assert_eq!(added.len(), 4);
        assert!(added.iter().all(|card| card.radiant == radiant));
    }

    #[test]
    fn r384_base_gains_mana_and_adds_four_cards_at_0() {
        opens(false);
    }

    #[test]
    fn r384_radiant_adds_four_radiant_cards_at_0() {
        opens(true);
    }
}

mod c_n76_1_brother_ping {
    use super::*;

    fn ping(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "field": [on(BROTHER_PING, radiant)], "hand": [STOCKPILE] },
            "p2": { "field": [MENACE], "hand": [FILLER] },
        }))
    }

    fn at_their_hero(action: &ActionBody) -> bool {
        targets_of(action) == vec![hero(P2)]
    }

    #[test]
    fn r384_base_activate_deals_1_once_a_turn() {
        let mut s = ping(false);
        activate_listed(&mut s, BROTHER_PING, at_their_hero);
        s.expect_health(P2, 29);
        let id = s.card(BROTHER_PING).id.clone();
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }

    #[test]
    fn r384_radiant_activate_2_deals_1_twice_a_turn() {
        let mut s = ping(true);
        activate_listed(&mut s, BROTHER_PING, at_their_hero);
        let menace = s.card(MENACE).id.clone();
        activate_listed(&mut s, BROTHER_PING, |action| {
            targets_of(action) == vec![instance(&menace)]
        });
        s.expect_health(P2, 29)
            .expect_stats(MENACE, json!({ "health": 8 }));
        let id = s.card(BROTHER_PING).id.clone();
        assert!(listed(&s, &id).is_empty(), "Activate 2 is twice per turn");
    }
}

/// The Meditative buildings (#97, MB25): p1's one on its field with a spare in hand and a library each,
/// so R82's auto-end stays away after its use.
fn building(card: &str, radiant: bool) -> Scenario {
    scenario(json!({
        "p1": { "field": [on(card, radiant)], "hand": [FILLER], "library": [X, X] },
        "p2": { "hand": [FILLER], "library": [X, X] },
    }))
}

fn upgrades_of(events: &[GameEvent], card: &str) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, GameEvent::Upgraded { instance_id, .. } if instance_id == card))
        .count()
}

mod m_n97_2_wishing_well {
    use super::*;

    fn sure(radiant: bool) -> Scenario {
        let mut s = building(WISHING_WELL, radiant);
        set_param(s.card_mut(WISHING_WELL), "chance", 100);
        s
    }

    #[test]
    fn r384_base_activate_adds_a_radiant_card_to_your_hand_once_a_turn() {
        let mut s = sure(false);
        let hand = s.hand(P1).len();
        activate_listed(&mut s, WISHING_WELL, any);
        assert_eq!(s.hand(P1).len(), hand + 1);
        assert!(
            s.hand(P1)
                .iter()
                .any(|card| card.radiant && card.def_id != FILLER)
        );
        let id = s.card(WISHING_WELL).id.clone();
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }

    #[test]
    fn r384_radiant_activate_adds_a_radiant_card_to_your_hand_once_a_turn() {
        let mut s = sure(true);
        let hand = s.hand(P1).len();
        activate_listed(&mut s, WISHING_WELL, any);
        assert_eq!(s.hand(P1).len(), hand + 1);
        let id = s.card(WISHING_WELL).id.clone();
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }
}

mod m_n97_3_school {
    use super::*;

    #[test]
    fn r384_base_activate_summons_a_unit_once_a_turn() {
        let mut s = building(SCHOOL, false);
        assert_eq!(units(&s, P1).len(), 1);
        activate_listed(&mut s, SCHOOL, any);
        let board = units(&s, P1);
        assert_eq!(board.len(), 2);
        assert!(board.iter().all(|unit| !unit.radiant));
        let id = s.card(SCHOOL).id.clone();
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }

    #[test]
    fn r384_radiant_activate_summons_a_radiant_unit_once_a_turn() {
        let mut s = building(SCHOOL, true);
        assert_eq!(units(&s, P1).len(), 1);
        activate_listed(&mut s, SCHOOL, any);
        let board = units(&s, P1);
        assert_eq!(board.len(), 2);
        assert!(board.iter().all(|unit| unit.radiant));
        let id = s.card(SCHOOL).id.clone();
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }
}

mod m_n97_4_mega_church {
    use super::*;

    fn church_scene(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "field": [on(MEGA_CHURCH, radiant)], "hand": [FILLER], "library": [X, X] },
            "p2": { "field": [on(VANILLA, false)], "hand": [FILLER], "library": [X, X] },
        }))
    }

    #[test]
    fn r384_base_activate_takes_an_enemy_permanent_once_a_turn() {
        let mut s = church_scene(false);
        let victim = s.card(VANILLA).id.clone();
        activate_listed(&mut s, MEGA_CHURCH, |action| {
            targets_of(action) == vec![instance(&victim)]
        });
        assert_eq!(s.card(VANILLA).controller, P1);
        let id = s.card(MEGA_CHURCH).id.clone();
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }

    #[test]
    fn r384_radiant_activate_takes_an_enemy_permanent_and_makes_it_radiant_once_a_turn() {
        let mut s = church_scene(true);
        let victim = s.card(VANILLA).id.clone();
        activate_listed(&mut s, MEGA_CHURCH, |action| {
            targets_of(action) == vec![instance(&victim)]
        });
        assert_eq!(s.card(VANILLA).controller, P1);
        assert!(s.card(VANILLA).radiant);
        let id = s.card(MEGA_CHURCH).id.clone();
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }
}

mod m_n97_6_university {
    use super::*;

    fn campus(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "field": [on(UNIVERSITY, radiant), VANILLA], "hand": [FILLER], "library": [X, X] },
            "p2": { "hand": [FILLER], "library": [X, X] },
        }))
    }

    #[test]
    fn r384_base_activate_buffs_the_other_permanent_twice_once_a_turn() {
        let mut s = campus(false);
        let id = s.card(UNIVERSITY).id.clone();
        let other = s.card(VANILLA).id.clone();
        let events = activate_listed(&mut s, UNIVERSITY, any);
        assert_eq!(upgrades_of(&events, &other), 2);
        assert_eq!(upgrades_of(&events, &id), 0);
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }

    #[test]
    fn r384_radiant_activate_buffs_the_other_permanent_four_times_once_a_turn() {
        let mut s = campus(true);
        let id = s.card(UNIVERSITY).id.clone();
        let other = s.card(VANILLA).id.clone();
        let events = activate_listed(&mut s, UNIVERSITY, any);
        assert_eq!(upgrades_of(&events, &other), 4);
        assert_eq!(upgrades_of(&events, &id), 0);
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }
}

mod m_n97_9_jlockeeds_headquarters {
    use super::*;

    #[test]
    fn r384_base_activate_fills_the_board_once_a_turn() {
        let mut s = building(HEADQUARTERS, false);
        activate_listed(&mut s, HEADQUARTERS, any);
        assert_eq!(units(&s, P1).len(), 5);
        let id = s.card(HEADQUARTERS).id.clone();
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }

    #[test]
    fn r384_radiant_activate_2_fills_the_board_and_is_listed_for_its_second_use() {
        let mut s = building(HEADQUARTERS, true);
        activate_listed(&mut s, HEADQUARTERS, any);
        assert_eq!(units(&s, P1).len(), 5);
        assert!(units(&s, P1).iter().skip(1).all(|unit| unit.radiant));
        let id = s.card(HEADQUARTERS).id.clone();
        assert!(!listed(&s, &id).is_empty(), "Activate 2 has a second use");
        activate_listed(&mut s, HEADQUARTERS, any);
        assert!(listed(&s, &id).is_empty(), "Activate 2 is twice per turn");
    }
}

mod m_n54_money_machine {
    use super::*;

    fn shuffles_a_coin_in(radiant: bool) {
        let mut s = scenario(json!({
            "p1": { "backrow": [on(MONEY_MACHINE, radiant)], "hand": [STOCKPILE], "library": [X, X] },
            "p2": { "hand": [FILLER] },
        }));
        let mana = s.state().players.p1.mana.current;
        activate_listed(&mut s, MONEY_MACHINE, any);
        let coins: Vec<CardInstance> = s
            .pile(P1, "library")
            .into_iter()
            .filter(|card| card.def_id == THE_COIN)
            .collect();
        assert_eq!(coins.len(), 1);
        assert_eq!(coins[0].radiant, radiant);
        assert_eq!(s.state().players.p1.mana.current, mana);
        s.expect_in_zone(MONEY_MACHINE, "field");
        let id = s.card(MONEY_MACHINE).id.clone();
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }

    #[test]
    fn r384_base_shuffles_a_coin_into_your_deck_once_a_turn() {
        shuffles_a_coin_in(false);
    }

    #[test]
    fn r384_radiant_shuffles_a_radiant_coin_into_your_deck_once_a_turn() {
        shuffles_a_coin_in(true);
    }

    /// (R384: only its controller activates it).
    #[test]
    fn r1105_the_opponent_is_not_listed_it_on_their_own_turn() {
        let s = scenario(json!({
            "active": "p2",
            "p1": { "backrow": [MONEY_MACHINE], "hand": [STOCKPILE] },
            "p2": { "hand": [FILLER] },
        }));
        let id = s.card(MONEY_MACHINE).id.clone();
        assert!(
            !legal_actions(s.state(), P2).into_iter().any(
                |action| matches!(action, ActionBody::Activate { instance_id, .. } if instance_id == id)
            )
        );
    }
}

mod m_n101_gachaholic {
    use super::*;

    fn gachaholic(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "field": [on(GACHAHOLIC, radiant)], "hand": [STOCKPILE] },
            "p2": { "hand": [FILLER] },
        }))
    }

    #[test]
    fn r384_base_has_no_activate() {
        let s = gachaholic(false);
        let id = s.card(GACHAHOLIC).id.clone();
        assert!(listed(&s, &id).is_empty(), "the base face's pull is a Cry");
    }

    #[test]
    fn r384_r1438_radiant_adds_a_luck_based_card_given_lucky_1_once_a_turn() {
        let mut s = gachaholic(true);
        activate_listed(&mut s, GACHAHOLIC, any);
        let pulled: Vec<CardInstance> = s
            .hand(P1)
            .into_iter()
            .filter(|card| card.def_id != STOCKPILE)
            .collect();
        assert_eq!(pulled.len(), 1);
        assert!(is_luck_based(&jackioh_cards::card_def(&pulled[0].def_id)));
        assert_eq!(pulled[0].granted_keywords, vec![Keyword::Lucky { n: 1 }]);
        let id = s.card(GACHAHOLIC).id.clone();
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }
}

mod m80_aluneth {
    use super::*;

    fn fielded(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "backrow": [on(ALUNETH, radiant)], "hand": [STOCKPILE] },
            "p2": { "hand": [FILLER] },
        }))
    }

    #[test]
    fn r384_base_has_no_activate() {
        let s = fielded(false);
        let id = s.card(ALUNETH).id.clone();
        assert!(
            listed(&s, &id).is_empty(),
            "only the Radiant face prints an Activate"
        );
    }

    #[test]
    fn r384_radiant_activate_exiles_it() {
        let mut s = fielded(true);
        let events = activate_listed(&mut s, ALUNETH, any);
        s.expect_in_zone(ALUNETH, "exile");
        // Exiled, not destroyed: no Death and no graveyard (R384).
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, GameEvent::EnteredGraveyard { .. }))
        );
    }
}

mod m_n49_2_yiles_virus {
    use super::*;

    /// p1's Virus on its field with three spares in hand, so a discard leaves R82's auto-end away.
    fn infected(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "field": [on(YILES_VIRUS, radiant)], "hand": [STOCKPILE, FILLER, FILLER], "library": [X, X] },
            "p2": { "hand": [FILLER], "library": [X, X] },
        }))
    }

    fn discards_of(events: &[GameEvent]) -> usize {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Discarded { .. }))
            .count()
    }

    #[test]
    fn r384_base_activate_destroys_it_and_its_death_discards_1() {
        let mut s = infected(false);
        let events = activate_listed(&mut s, YILES_VIRUS, any);
        assert!(units(&s, P1).is_empty(), "the Virus destroyed itself");
        assert_eq!(discards_of(&events), 1);
        assert_eq!(s.hand(P1).len(), 2);
    }

    #[test]
    fn r384_radiant_activate_destroys_it_and_its_death_discards_2() {
        let mut s = infected(true);
        let events = activate_listed(&mut s, YILES_VIRUS, any);
        assert!(units(&s, P1).is_empty(), "the Virus destroyed itself");
        assert_eq!(discards_of(&events), 2);
        assert_eq!(s.hand(P1).len(), 1);
    }
}

mod m_n69_the_maestro {
    use super::*;

    fn conducting(radiant: bool, counters: i32) -> Scenario {
        scenario(json!({
            "p1": {
                "field": [{ "def": MAESTRO, "radiant": radiant, "counters": { "plague": counters } }],
                "hand": [STOCKPILE],
                "library": [X, X],
            },
            "p2": {
                "field": [VANILLA, VANILLA],
                "hand": [FILLER, FILLER, FILLER],
                "library": [X, X, X],
                "graveyard": [FILLER, FILLER],
            },
        }))
    }

    fn plague(s: &Scenario) -> i32 {
        s.card(MAESTRO).counters.plague.unwrap_or(0)
    }

    fn enemy_zones(s: &Scenario) -> [usize; 4] {
        [
            units(s, P2).len(),
            s.hand(P2).len(),
            s.pile(P2, "library").len(),
            s.pile(P2, "graveyard").len(),
        ]
    }

    #[test]
    fn r384_it_is_not_listed_below_its_price() {
        let s = conducting(false, 3);
        let id = s.card(MAESTRO).id.clone();
        assert!(listed(&s, &id).is_empty(), "the Activate needs 4 Plague Counters");
    }

    #[test]
    fn r384_base_activate_spends_4_counters_and_exiles_1_from_each_enemy_zone_once_a_turn() {
        let mut s = conducting(false, 8);
        activate_listed(&mut s, MAESTRO, any);
        assert_eq!(plague(&s), 4);
        assert_eq!(enemy_zones(&s), [1, 2, 2, 1]);
        let id = s.card(MAESTRO).id.clone();
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }

    #[test]
    fn r384_radiant_activate_spends_4_counters_and_exiles_2_from_each_enemy_zone_once_a_turn() {
        let mut s = conducting(true, 8);
        activate_listed(&mut s, MAESTRO, any);
        assert_eq!(plague(&s), 4);
        assert_eq!(enemy_zones(&s), [0, 1, 1, 0]);
        let id = s.card(MAESTRO).id.clone();
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }
}

mod m_n75_ever_growing_tree {
    use super::*;

    /// p1's Tree in its backrow with a spare in hand. Fused, its def id is the fusion's (R102), so
    fn planted(radiant: bool) -> (Scenario, String) {
        let s = scenario(json!({
            "p1": { "backrow": [on(EVER_GROWING_TREE, radiant)], "hand": [STOCKPILE], "library": [X, X] },
            "p2": { "hand": [FILLER], "library": [X, X] },
        }));
        let id = s.card(EVER_GROWING_TREE).id.clone();
        (s, id)
    }

    fn grow(s: &mut Scenario) -> Vec<GameEvent> {
        activate_listed(s, EVER_GROWING_TREE, any);
        let offered: Vec<String> = s
            .state()
            .pending
            .as_ref()
            .expect("the Tree's choice")
            .options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Mode { option } => Some(option.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(offered.len(), 3);
        s.answer(json!(offered[0]));
        s.last_events().to_vec()
    }

    fn grown(s: &Scenario, id: &str) -> String {
        find_instance(s.state(), id)
            .expect("the Tree is still on the field")
            .def_id
            .clone()
    }

    #[test]
    fn r384_base_activate_fuses_a_field_spell_it_offers_into_it_once_a_turn() {
        let (mut s, id) = planted(false);
        let events = grow(&mut s);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, GameEvent::Fused { .. }))
        );
        assert_ne!(grown(&s, &id), EVER_GROWING_TREE);
        assert!(!grown(&s, &id).contains('*'), "the base face fuses a base pick");
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }

    #[test]
    fn r384_radiant_activate_fuses_a_radiant_field_spell_it_offers_into_it_once_a_turn() {
        let (mut s, id) = planted(true);
        let events = grow(&mut s);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, GameEvent::Fused { .. }))
        );
        // R469's mark: the pick went in Radiant.
        assert!(
            grown(&s, &id).contains('*'),
            "the fused id names a Radiant ingredient"
        );
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }
}

mod m_plus_n11_living_mana_potion {
    use super::*;

    /// The listed `activate`: p1 has Spell Damage +5 this turn (R1180001); the base face Tributes the
    fn activates(radiant: bool) {
        let mut s = scenario(json!({
            "p1": { "field": [on(LIVING_MANA_POTION, radiant)], "hand": [STOCKPILE] },
            "p2": { "hand": [FILLER] },
        }));
        let id = s.card(LIVING_MANA_POTION).id.clone();
        activate_listed(&mut s, LIVING_MANA_POTION, any);
        assert_eq!(jackioh_engine::damage::spell_damage_of(s.state(), P1), 5);
        if radiant {
            s.expect_in_zone(id.as_str(), "field");
            assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
        } else {
            s.expect_in_zone(id.as_str(), "graveyard");
        }
    }

    #[test]
    fn r384_r1180001_base_tributes_itself_for_spell_damage_5_this_turn() {
        activates(false);
    }

    #[test]
    fn r384_r1180001_radiant_stays_for_spell_damage_5_this_turn() {
        activates(true);
    }
}

mod m_plus_n12_living_armor_potion {
    use super::*;

    /// The listed `activate`: p1's hero has 5 Armor (R757); the base face Tributes the potion,
    fn activates(radiant: bool) {
        let mut s = scenario(json!({
            "p1": { "field": [on(LIVING_ARMOR_POTION, radiant)], "hand": [STOCKPILE] },
            "p2": { "hand": [FILLER] },
        }));
        let id = s.card(LIVING_ARMOR_POTION).id.clone();
        activate_listed(&mut s, LIVING_ARMOR_POTION, any);
        assert_eq!(jackioh_engine::damage::hero_armor_of(s.state(), P1), 5);
        if radiant {
            s.expect_in_zone(id.as_str(), "field");
            assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
        } else {
            s.expect_in_zone(id.as_str(), "graveyard");
        }
    }

    #[test]
    fn r384_base_tributes_itself_for_5_armor() {
        activates(false);
    }

    #[test]
    fn r384_radiant_stays_for_5_armor() {
        activates(true);
    }
}

mod m_plus_n13_living_health_potion {
    use super::*;

    /// The listed `activate`: p1's hero heals 5 (R1179002); the base face Tributes the potion,
    fn activates(radiant: bool) {
        let mut s = scenario(json!({
            "p1": { "health": 20, "field": [on(LIVING_HEALTH_POTION, radiant)], "hand": [STOCKPILE] },
            "p2": { "hand": [FILLER] },
        }));
        let id = s.card(LIVING_HEALTH_POTION).id.clone();
        activate_listed(&mut s, LIVING_HEALTH_POTION, any);
        assert_eq!(health(&s, P1), 25);
        if radiant {
            s.expect_in_zone(id.as_str(), "field");
            assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
        } else {
            s.expect_in_zone(id.as_str(), "graveyard");
        }
    }

    #[test]
    fn r384_base_tributes_itself_and_heals_your_hero_5() {
        activates(false);
    }

    #[test]
    fn r384_radiant_stays_and_heals_your_hero_5() {
        activates(true);
    }
}

/// The Stinkyscape set's Activate cards, from its release (#1884, R1857000): each Activate is found in
mod stinkyscape {
    use super::*;

    /// A card's declared number on a face (R386), as the catalog prints it.
    fn printed(def: &str, key: &str, radiant: bool) -> i32 {
        let card = &jackioh_cards::CATALOG[def];
        let param = card
            .params
            .iter()
            .flatten()
            .find(|param| param.key == key)
            .unwrap_or_else(|| panic!("{def} declares {key}"));
        if radiant { param.radiant } else { param.base }
    }

    fn hits_on(events: &[GameEvent], target: &str) -> Vec<i32> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage {
                    target_id, amount, ..
                } if target_id == target => Some(*amount),
                _ => None,
            })
            .collect()
    }

    fn summoned_defs(events: &[GameEvent]) -> Vec<String> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned {
                    player: PlayerId::P1,
                    def_id,
                    ..
                } => Some(def_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn armed(def: &str, radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "field": [on(def, radiant)], "hand": [STOCKPILE], "library": [X, X] },
            "p2": { "field": [MENACE], "hand": [FILLER], "library": [X, X] },
        }))
    }

    fn at_the_menace(s: &Scenario) -> impl Fn(&ActionBody) -> bool + use<> {
        let menace = s.card(MENACE).id.clone();
        move |action: &ActionBody| targets_of(action) == vec![instance(&menace)]
    }

    /// R1862000: a weapon that hits one chosen enemy deals its declared damage to the Menace.
    fn shoots_one(def: &str, radiant: bool) {
        let mut s = armed(def, radiant);
        let menace = s.card(MENACE).id.clone();
        let pick = at_the_menace(&s);
        let events = activate_listed(&mut s, def, pick);
        let hits = hits_on(&events, &menace);
        assert_eq!(
            hits.first().copied(),
            Some(printed(def, "damage", radiant)),
            "{def}: {hits:?}"
        );
    }

    /// R1862000, R2521001: a Flak weapon hits every enemy Unit, never the hero since #2657, `times` times,
    /// each hit a roll of its own between its declared least and most.
    fn shoots_all(def: &str, radiant: bool, times: usize) {
        let mut s = armed(def, radiant);
        let events = activate_listed(&mut s, def, any);
        let rolls = printed(def, "least", radiant)..=printed(def, "most", radiant);
        let menace = s.card(MENACE).id.clone();
        let hits = hits_on(&events, &menace);
        assert_eq!(hits.len(), times, "{def} on {menace}: {hits:?}");
        assert!(
            hits.iter().all(|hit| rolls.contains(hit)),
            "{def} on {menace}: {hits:?} not in {rolls:?}"
        );
        assert!(hits_on(&events, "hero-p2").is_empty(), "{def} hit the hero");
    }

    /// R1840000, R2250000: a Miner gains its controller a Resource into their hand.
    fn mines(def: &str, radiant: bool) {
        let mut s = armed(def, radiant);
        let before = resources_in_hand_of(s.state(), PlayerId::P1).len();
        activate_listed(&mut s, def, any);
        let held = resources_in_hand_of(s.state(), PlayerId::P1);
        assert_eq!(held.len(), before + 1, "{def}");
        let gained = held.last().expect("a Resource");
        assert!(
            jackioh_cards::CATALOG[&gained.def_id].resource.is_some(),
            "{}",
            gained.def_id
        );
        assert_eq!(gained.radiant, radiant, "{def}");
    }

    macro_rules! both_faces {
        ($name:ident, $check:expr) => {
            mod $name {
                use super::*;

                #[test]
                fn r384_base_through_the_listed_activate() {
                    ($check)(false);
                }

                #[test]
                fn r384_radiant_through_the_listed_activate() {
                    ($check)(true);
                }
            }
        };
    }

    const MENACE: &str = "core-019";

    both_faces!(s_n50_beam_v3, |radiant| shoots_one("stinkyscape-050", radiant));
    both_faces!(s_n51_repeater_v3, |radiant| shoots_one(
        "stinkyscape-051",
        radiant
    ));
    both_faces!(s_n52_cannon_v3, |radiant| shoots_one("stinkyscape-052", radiant));
    both_faces!(s_n53_flak_v3, |radiant| shoots_all("stinkyscape-053", radiant, 1));
    both_faces!(s_n54_beam_v4, |radiant| shoots_one("stinkyscape-054", radiant));
    both_faces!(s_n55_repeater_v4, |radiant| shoots_one(
        "stinkyscape-055",
        radiant
    ));
    both_faces!(s_n56_cannon_v4, |radiant| shoots_one("stinkyscape-056", radiant));
    both_faces!(s_n57_flak_v4, |radiant| shoots_all("stinkyscape-057", radiant, 2));
    both_faces!(s_n58_beam_v5, |radiant| shoots_one("stinkyscape-058", radiant));
    both_faces!(s_n59_repeater_v5, |radiant| shoots_one(
        "stinkyscape-059",
        radiant
    ));
    both_faces!(s_n60_cannon_v5, |radiant| shoots_one("stinkyscape-060", radiant));
    both_faces!(s_n61_flak_v5, |radiant| shoots_all("stinkyscape-061", radiant, 3));
    both_faces!(s_n62_miner_v1, |radiant| mines("stinkyscape-062", radiant));
    both_faces!(s_n63_miner_v2, |radiant| mines("stinkyscape-063", radiant));
    both_faces!(s_n64_miner_v3, |radiant| mines("stinkyscape-064", radiant));
    both_faces!(s_n65_miner_v4, |radiant| mines("stinkyscape-065", radiant));
    both_faces!(s_n66_miner_v5, |radiant| mines("stinkyscape-066", radiant));
    both_faces!(s_n142_jade_launcher, |radiant| shoots_one(
        "stinkyscape-142",
        radiant
    ));

    both_faces!(s_n140_rush_token_launcher, |radiant: bool| {
        let mut s = armed("stinkyscape-140", radiant);
        let events = activate_listed(&mut s, "stinkyscape-140", any);
        assert_eq!(summoned_defs(&events), vec!["core-t-rush".to_string()]);
        assert!(!hits_on(&events, &s.card(MENACE).id).is_empty());
    });

    both_faces!(s_n141_fruit_launcher, |radiant: bool| {
        let mut s = armed("stinkyscape-141", radiant);
        let events = activate_listed(&mut s, "stinkyscape-141", any);
        let fruit: Vec<&String> = events
            .iter()
            .filter_map(|event| match event {
                GameEvent::CardPlayed { def_id, .. } => Some(def_id),
                _ => None,
            })
            .filter(|def| jackioh_cards::CATALOG[def.as_str()].tags.contains(&Tag::Fruit))
            .collect();
        assert_eq!(fruit.len(), 1, "one random Fruit cast: {events:?}");
    });

    both_faces!(s_n143_book_launcher, |radiant: bool| {
        let mut s = armed("stinkyscape-143", radiant);
        let events = activate_listed(&mut s, "stinkyscape-143", any);
        let books = events
            .iter()
            .filter(|event| {
                matches!(event, GameEvent::CardPlayed { def_id, .. }
                    if jackioh_cards::CATALOG[def_id.as_str()].tags.contains(&Tag::Book))
            })
            .count();
        assert_eq!(books, 1, "one random Book cast: {events:?}");
    });

    both_faces!(s_n175_frost_module, |radiant: bool| {
        let mut s = armed("stinkyscape-175", radiant);
        let pick = at_the_menace(&s);
        activate_listed(&mut s, "stinkyscape-175", pick);
        assert!(s.card(MENACE).frozen.is_some(), "the Menace is Frozen");
    });

    both_faces!(s_n125_jlockeeds_spacerace_propaganda, |radiant: bool| {
        let mut s = scenario(json!({
            "p1": { "backrow": [on("stinkyscape-125", radiant)], "hand": [STOCKPILE], "library": [X, X] },
            "p2": { "hand": [FILLER], "library": [X, X] },
        }));
        activate_listed(&mut s, "stinkyscape-125", any);
        let added: Vec<CardInstance> = s
            .hand(P1)
            .into_iter()
            .filter(|card| card.def_id != STOCKPILE)
            .collect();
        assert_eq!(added.len(), 1);
        let piece = &jackioh_cards::CATALOG[&added[0].def_id];
        assert!(
            piece
                .base
                .keywords
                .iter()
                .any(|keyword| keyword.kind() == KeywordKind::StarshipPiece),
            "{} is a Starship Piece",
            piece.id
        );
        assert_eq!(added[0].radiant, radiant);
    });

    both_faces!(s_t_propagation_layer, |radiant: bool| {
        let mut s = scenario(json!({
            "p1": { "field": [on("stinkyscape-t-propagation-layer", radiant)], "hand": [STOCKPILE], "library": [X, X] },
            "p2": { "hand": [FILLER], "library": [X, X] },
        }));
        let events = activate_listed(&mut s, "stinkyscape-t-propagation-layer", |action| {
            ability_of(action).is_some_and(|ability| ability.contains("corvette"))
        });
        assert_eq!(
            summoned_defs(&events),
            vec!["stinkyscape-t-drone-corvette".to_string()]
        );
    });

    both_faces!(s_n18_1_melchior, |radiant: bool| {
        let mut s = scenario(json!({
            "p1": { "backrow": [on("stinkyscape-018-1", radiant)], "field": [VANILLA], "hand": [STOCKPILE], "library": [X, X] },
            "p2": { "hand": [FILLER], "library": [X, X] },
        }));
        let vanilla = s.card(VANILLA).id.clone();
        activate_listed(&mut s, "stinkyscape-018-1", any);
        if !radiant {
            let options = s
                .state()
                .pending
                .as_ref()
                .map(|pending| pending.options.clone())
                .unwrap_or_default();
            assert_eq!(options.len(), 3);
            s.answer(json!(options[0].key.clone()));
        }
        assert!(
            !s.card(&vanilla).granted_keywords.is_empty(),
            "the Vanilla gained a keyword"
        );
    });

    both_faces!(s_n18_2_balthasar, |radiant: bool| {
        let mut s = scenario(json!({
            "p1": { "backrow": [on("stinkyscape-018-2", radiant)], "hand": [STOCKPILE, STOCKPILE], "library": [X, X, X] },
            "p2": { "hand": [FILLER], "library": [X, X] },
        }));
        s.play(STOCKPILE, json!({}));
        let hand = s.hand(P1).len();
        activate_listed(&mut s, "stinkyscape-018-2", any);
        assert_eq!(s.hand(P1).len(), hand + 1, "a copy of the last card played");
        let copy = s.hand(P1).into_iter().last().expect("the copy");
        assert_eq!(copy.def_id, STOCKPILE);
        assert_eq!(copy.radiant, radiant);
    });

    both_faces!(s_n18_3_casper, |radiant: bool| {
        let mut s = scenario(json!({
            "p1": { "backrow": [on("stinkyscape-018-3", radiant)], "hand": [STOCKPILE], "library": [X, X] },
            "p2": { "field": [VANILLA], "hand": [VANILLA], "library": [X, X] },
        }));
        activate_listed(&mut s, "stinkyscape-018-3", any);
        // R1866004: the Allure on the field is a delayed steal watching the enemy Vanilla.
        let vanilla = s.unit(P2, 1).expect("the enemy Vanilla").id;
        let watched: Vec<String> = s
            .state()
            .delayed
            .iter()
            .filter_map(|entry| entry.watch.clone())
            .collect();
        assert!(watched.contains(&vanilla), "{watched:?}");
    });

    both_faces!(s_n26_zolarketh, |radiant: bool| {
        let mut s = scenario(json!({
            "p1": { "field": [on("stinkyscape-026", radiant)], "hand": [STOCKPILE], "library": [X, X] },
            "p2": { "hand": [FILLER], "library": [X, X] },
        }));
        // #2657: its Activate is its own once its Cry chose it, which it remembers.
        s.card_mut("stinkyscape-026")
            .memory
            .insert("gainedPieceActivate".to_string(), json!(true));
        let events = activate_listed(&mut s, "stinkyscape-026", |_| true);
        let pieces = summoned_defs(&events);
        assert_eq!(pieces.len(), 1, "{events:?}");
        assert!(
            jackioh_cards::CATALOG[&pieces[0]]
                .base
                .keywords
                .iter()
                .any(|keyword| keyword.kind() == KeywordKind::StarshipPiece)
        );
    });

    mod s_n157_the_gauntlet {
        use super::*;

        #[test]
        fn r384_radiant_through_the_listed_activate() {
            let mut s = scenario(json!({
                "p1": { "backrow": [on("stinkyscape-157", true)], "hand": [STOCKPILE], "library": [X, X] },
                "p2": { "hand": [FILLER], "library": [X, X] },
            }));
            let id = s.card("stinkyscape-157").id.clone();
            activate_listed(&mut s, "stinkyscape-157", |action| {
                modes_of(action).iter().any(|mode| mode.contains("Tribute"))
            });
            s.expect_in_zone(id.as_str(), "graveyard");
        }

        #[test]
        fn r384_base_offers_no_activate() {
            let s = scenario(json!({
                "p1": { "backrow": [on("stinkyscape-157", false)], "hand": [STOCKPILE], "library": [X, X] },
                "p2": { "hand": [FILLER], "library": [X, X] },
            }));
            assert!(listed(&s, &s.card("stinkyscape-157").id).is_empty());
        }
    }

    both_faces!(s_n112_job_application, |radiant: bool| {
        let mut s = scenario(json!({
            "p1": { "backrow": [on("stinkyscape-112", radiant)], "hand": [STOCKPILE, FILLER], "library": [X, X] },
            "p2": { "hand": [FILLER], "library": [X, X] },
        }));
        let hand = s.hand(P1).len();
        // Stage 1: the resume, a card from the hand sent in (R1865000).
        activate_listed(&mut s, "stinkyscape-112", any);
        assert_eq!(s.hand(P1).len(), hand - 1, "the resume left the hand");
    });

    both_faces!(s_n148_alteran_reach, |radiant: bool| {
        let mut s = scenario(json!({
            "p1": { "backrow": [on("stinkyscape-148", radiant)], "hand": [STOCKPILE], "library": [X, X], "mana": 4 },
            "p2": { "hand": [FILLER], "library": [X, X] },
        }));
        let mana = s.state().players.p1.mana.current;
        // A travel costs (1); the Radiant face's first each turn is free (R1847001).
        activate_listed(&mut s, "stinkyscape-148", any);
        assert_eq!(
            s.state().players.p1.mana.current,
            if radiant { mana } else { mana - 1 }
        );
    });

    mod s_t_dormant_starship {
        use super::*;

        /// R1852003: a Starship Piece's death summons the Dormant Starship, whose launch is an Activate
        #[test]
        fn r384_its_launch_through_the_listed_activate() {
            const HIT_JOB: &str = "core-016";
            const AP_ROUNDS: &str = "stinkyscape-100"; // (1) 3/1 Starship Piece, Equip, Pierce.
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": AP_ROUNDS, "lane": 2 }], "hand": [HIT_JOB, STOCKPILE], "library": [X, X], "mana": 10 },
                "p2": { "hand": [FILLER], "library": [X, X] },
            }));
            let targets = pick::unit_at(&s, P1, 2);
            s.play(HIT_JOB, json!({ "targets": targets }));
            let ship = jackioh_engine::prelude::dormant_starship_of(s.state(), P1)
                .expect("the piece's death summoned the Starship")
                .id
                .clone();
            let mana = s.state().players.p1.mana.current;
            activate_listed(&mut s, &ship, any);
            assert_eq!(s.state().players.p1.mana.current, mana - 3);
        }
    }

    /// R1840006: a mining node Tributes itself through its Activate, and its Death pays.
    fn mined(def: &str, radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "field": [on(def, radiant)], "hand": [STOCKPILE], "library": [X, X] },
            "p2": { "hand": [FILLER], "library": [X, X] },
        }));
        let id = s.card(def).id.clone();
        activate_listed(&mut s, def, any);
        s.expect_in_zone(id.as_str(), "graveyard");
        s
    }

    /// R2250000: the Resources in p1's hand.
    fn tray_len(s: &Scenario) -> usize {
        resources_in_hand_of(s.state(), PlayerId::P1).len()
    }

    both_faces!(s_n169_tiny_asteroid, |radiant: bool| {
        let s = mined("stinkyscape-169", radiant);
        assert_eq!(
            tray_len(&s) as i32,
            printed("stinkyscape-169", "resources", radiant)
        );
    });
    both_faces!(s_n170_medium_asteroid, |radiant: bool| {
        let s = mined("stinkyscape-170", radiant);
        assert_eq!(
            tray_len(&s) as i32,
            printed("stinkyscape-170", "resources", radiant)
        );
    });
    both_faces!(s_n171_large_asteroid, |radiant: bool| {
        let s = mined("stinkyscape-171", radiant);
        assert_eq!(
            tray_len(&s) as i32,
            printed("stinkyscape-171", "resources", radiant)
        );
    });
    both_faces!(s_n172_huge_asteroid, |radiant: bool| {
        let s = mined("stinkyscape-172", radiant);
        assert_eq!(
            tray_len(&s) as i32,
            printed("stinkyscape-172", "resources", radiant)
        );
    });
    both_faces!(s_n173_abandoned_spice_tower, |radiant: bool| {
        let s = mined("stinkyscape-173", radiant);
        let spices = s
            .hand(P1)
            .into_iter()
            .filter(|card| card.def_id == "stinkyscape-089")
            .count();
        assert_eq!(spices as i32, printed("stinkyscape-173", "spices", radiant));
    });

    both_faces!(s_n116_megabase, |radiant: bool| {
        let mut s = scenario(json!({
            "p1": { "field": [on("stinkyscape-116", radiant)], "hand": [STOCKPILE], "library": [X, X] },
            "p2": { "hand": [FILLER], "library": [X, X] },
        }));
        activate_listed(&mut s, "stinkyscape-116", any);
        let credits = |player: PlayerId| s.state().players[player].market_credits.unwrap_or(0);
        assert!(credits(P1) > 0, "its controller gains Credits");
        assert_eq!(credits(P2), if radiant { credits(P1) - 5 } else { credits(P1) });
    });

    both_faces!(s_n39_the_narcor_project, |radiant: bool| {
        let mut s = scenario(json!({
            "p1": { "backrow": [on("stinkyscape-039", radiant)], "hand": [STOCKPILE], "library": [X, X, X, X] },
            "p2": { "hand": [FILLER], "library": [X, X, X, X] },
        }));
        assert!(listed(&s, &s.card("stinkyscape-039").id).is_empty());
        s.end_turn().end_turn();
        let held = tray_len(&s);
        assert!(held >= 1, "its end of turn gained a Narcor");
        activate_listed(&mut s, "stinkyscape-039", any);
        assert_eq!(tray_len(&s), held - 1, "a Narcor discarded");
    });
}
