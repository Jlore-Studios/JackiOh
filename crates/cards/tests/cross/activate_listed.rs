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
const TWINSPELL: &str = "core-079";
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
const FENG_SHUI: &str = "meditative-040";
const MONEY_MACHINE: &str = "meditative-054";
const GACHAHOLIC: &str = "meditative-101";
const ALUNETH: &str = "meditative-080";
const YILES_VIRUS: &str = "meditative-049-2";
const MAESTRO: &str = "meditative-069";
const EVER_GROWING_TREE: &str = "meditative-075";
const LIVING_MANA_POTION: &str = "meditativeplus-011";
const LIVING_BOMB: &str = "classic-062";
const UNBIASED_IMMIGRATION: &str = "core-059";
const LIVING_ARMOR_POTION: &str = "meditativeplus-012";
const LIVING_HEALTH_POTION: &str = "meditativeplus-013";

const ACTIVATE_CARDS: [&str; 99] = [
    HEROIC_POWER,
    TWINSPELL,
    UNBIASED_IMMIGRATION,
    INFINISCEPTER,
    NOSE_HUNTER,
    POWER_TO_PUNISH,
    TURTINATOR,
    DEVILS_PACT,
    TRANSMUTABLE_TOXINS,
    LIVING_BOMB,
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
    FENG_SHUI,
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
    "stinkyscape-168",
    "stinkyscape-169",
    "stinkyscape-170",
    "stinkyscape-171",
    "stinkyscape-172",
    "stinkyscape-173",
    "stinkyscape-175",
    "stinkyscape-t-dormant-starship",
    "stinkyscape-t-propagation-layer",
    "stinkyscapeplus-004",
    "stinkyscapeplus-008",
    "stinkyscapeplus-009",
    "stinkyscapeplus-010",
    "stinkyscapeplus-011",
    "stinkyscapeplus-019",
    "stinkyscapeplus-029",
    "stinkyscapeplus-035",
    "stinkyscapeplus-038",
    "stinkyscapeplus-055",
    "stinkyscapeplus-059",
    "stinkyscapeplus-109",
    "stinkyscapeplus-124",
    "stinkyscapeplus-125",
    "stinkyscapeplus-126",
    "stinkyscapeplus-127",
    "stinkyscapeplus-136",
    "stinkyscapeplus-143",
    "stinkyscapeplus-144",
    "stinkyscapeplus-145",
    "stinkyscapeplus-146",
    "stinkyscapeplus-147",
    "stinkyscapeplus-148",
    "stinkyscapeplus-157",
    "stinkyscapeplus-160",
];

const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2. A spare that keeps R82's auto-end away.
const MANA_WELL: &str = "core-006"; // Mana Well: a backrow card of p1's to hold a Plague Counter
const VANILLA: &str = "core-008"; // Mr. Vanilla
const FILLER: &str = "core-010"; // Rapid Replenish: a Spell
const MENACE: &str = "core-019"; // Midrange Menace: Taunt Unit
const X: &str = "core-020"; // Pointmaster: library filler
const CONJURE_KY: &str = "core-057"; // Conjure KY: Add 3 random KY cards to your hand
const APPLE: &str = "classicplus-059"; // All Purpose Apple: Doctors Orders' card
const THE_COIN: &str = "core-t-coin"; // The Coin: Money Machine's card
const AUSPICIOUS_ROCK: &str = "meditative-039-1"; // Auspicious Rock: Feng Shui's card

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

/// The Activates the engine gives a card rather than its script: the Dormant Starship's launch
/// (R1852003) and Alteran Reach's travel (`subsystems::starscape_abilities`, R1847001).
const ENGINE_ACTIVATES: [&str; 2] = ["stinkyscape-t-dormant-starship", "stinkyscape-148"];

/// The cards whose text names an Activate without printing one: S+ #43 Segment counters "the first enemy
/// Ranged attack or Activate per turn" (SPEC §8.12 row 43), an answer to other cards' abilities.
const NAMES_AN_ACTIVATE: [&str; 1] = ["stinkyscapeplus-043"];

/// The census: the cards this file covers are exactly the cards whose text has an Activate, which are
/// exactly the cards whose script declares one, among the sets that ship (a held set's join at its
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
        if (def.base.text.contains("Activate") || def.radiant.text.contains("Activate"))
            && !NAMES_AN_ACTIVATE.contains(&id.as_str())
        {
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

/// Issue #3139: Core #59's Activate (both faces), once a turn with no price, as a client uses it.
mod c_n59_unbiased_immigration {
    use super::*;

    fn standing(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "backrow": [on(UNBIASED_IMMIGRATION, radiant)], "hand": [STOCKPILE], "library": [X, X] },
            "p2": { "hand": [FILLER], "library": [X, X] },
        }))
    }

    fn adds_a_card(radiant: bool) {
        let mut s = standing(radiant);
        let id = s.card(UNBIASED_IMMIGRATION).id.clone();
        let hand = s.hand(P1).len();
        let mana = s.state().players.p1.mana.current;
        let events = activate_listed(&mut s, UNBIASED_IMMIGRATION, any);
        assert_eq!(s.hand(P1).len(), hand + 1);
        assert_eq!(s.state().players.p1.mana.current, mana, "no price");
        let added: Vec<&GameEvent> = events
            .iter()
            .filter(|event| matches!(event, GameEvent::AddedToHand { .. }))
            .collect();
        assert_eq!(added.len(), 1);
        let GameEvent::AddedToHand { instance_id, .. } = added[0] else {
            unreachable!("filtered to an add")
        };
        assert_eq!(s.card(instance_id.as_str()).radiant, radiant);
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }

    #[test]
    fn r384_base_activate_adds_a_random_card_once_a_turn() {
        adds_a_card(false);
    }

    #[test]
    fn r384_radiant_activate_adds_a_random_radiant_card_once_a_turn() {
        adds_a_card(true);
    }
}

/// Issue #3139: Classic #62's Radiant face Activates a Plague Counter (R689's prompt), once a turn; the base
/// face has the Cry instead and lists no Activate.
mod c_n62_living_bomb {
    use super::*;

    fn standing(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "backrow": [on(LIVING_BOMB, radiant)], "field": [VANILLA], "hand": [STOCKPILE], "library": [X, X] },
            "p2": { "field": [MENACE], "hand": [FILLER], "library": [X, X] },
        }))
    }

    #[test]
    fn r384_r689_radiant_activate_places_a_plague_counter_on_the_permanent_a_prompt_names_once_a_turn() {
        let mut s = standing(true);
        let id = s.card(LIVING_BOMB).id.clone();
        activate_listed(&mut s, LIVING_BOMB, any);
        let prey = s.card(MENACE).id.clone();
        assert!(
            s.state().pending.is_some(),
            "R689: one prompt names the permanent"
        );
        s.answer(json!(prey));
        assert_eq!(s.card(MENACE).counters.plague, Some(1));
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }

    #[test]
    fn r384_the_base_face_lists_no_activate() {
        let s = standing(false);
        let id = s.card(LIVING_BOMB).id.clone();
        assert!(listed(&s, &id).is_empty());
    }
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
        // R3551002: Life Tap costs (0), so the 3 mana stays.
        s.expect_mana(P1, 3);
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
        // R3551002: Life Tap costs (0), so the 3 mana stays.
        s.expect_mana(P1, 3);
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
        // A base Book since #2179: 4, raised by its Spell Damage +2 (R1563002).
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
    /// card into p1's hand costing (0), Radiant on the Radiant face, the Datacenter staying, once a turn.
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
    /// its controller the declared mana and adds the four cards, which cost (0).
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

mod m_n40_feng_shui {
    use super::*;

    /// R384, R3140000: the listed `activate` adds an Auspicious Rock, Radiant on the Radiant face, once a turn.
    fn adds_a_rock(radiant: bool) {
        let mut s = scenario(json!({
            "p1": { "backrow": [on(FENG_SHUI, radiant)], "hand": [STOCKPILE], "library": [X, X] },
            "p2": { "hand": [FILLER] },
        }));
        let mana = s.state().players.p1.mana.current;
        activate_listed(&mut s, FENG_SHUI, any);
        let rocks: Vec<CardInstance> = s
            .hand(P1)
            .into_iter()
            .filter(|card| card.def_id == AUSPICIOUS_ROCK)
            .collect();
        assert_eq!(rocks.len(), 1);
        assert_eq!(rocks[0].radiant, radiant);
        assert_eq!(s.state().players.p1.mana.current, mana);
        s.expect_in_zone(FENG_SHUI, "field");
        let id = s.card(FENG_SHUI).id.clone();
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }

    #[test]
    fn r384_r3140000_base_adds_an_auspicious_rock_to_your_hand_once_a_turn() {
        adds_a_rock(false);
    }

    #[test]
    fn r384_r3140000_radiant_adds_a_radiant_auspicious_rock_to_your_hand_once_a_turn() {
        adds_a_rock(true);
    }

    #[test]
    fn r384_the_opponent_is_not_listed_it_on_their_own_turn() {
        let s = scenario(json!({
            "active": "p2",
            "p1": { "backrow": [FENG_SHUI], "hand": [STOCKPILE] },
            "p2": { "hand": [FILLER] },
        }));
        let id = s.card(FENG_SHUI).id.clone();
        assert!(
            !legal_actions(s.state(), P2).into_iter().any(
                |action| matches!(action, ActionBody::Activate { instance_id, .. } if instance_id == id)
            )
        );
    }
}

mod m_n54_money_machine {
    use super::*;

    /// R3551001, R767000: since S01 the Machine prints its card with its Cry and has no Activate (so the
    /// census holds it no more); the script kept for the versions before it still lists one, which this
    /// module drives, folding under that script.
    fn under_kept<T>(run: impl FnOnce() -> T) -> T {
        jackioh_cards::register_all();
        let (_, _, kept) = jackioh_cards::KEPT_SCRIPTS
            .iter()
            .rev()
            .find(|(id, _, _)| *id == MONEY_MACHINE)
            .expect("Money Machine's Activate script is kept");
        let shipped: &'static jackioh_engine::ShippedCatalog =
            Box::leak(Box::new(jackioh_engine::ShippedCatalog::new(
                "kept",
                jackioh_cards::CATALOG.clone(),
                indexmap::IndexMap::from([(MONEY_MACHINE.to_string(), kept())]),
            )));
        jackioh_engine::with_shipped(Some(shipped), run)
    }

    fn shuffles_a_coin_in(radiant: bool) {
        under_kept(|| {
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
        })
    }

    #[test]
    fn r384_base_shuffles_a_coin_into_your_deck_once_a_turn() {
        shuffles_a_coin_in(false);
    }

    #[test]
    fn r384_radiant_shuffles_a_radiant_coin_into_your_deck_once_a_turn() {
        shuffles_a_coin_in(true);
    }

    /// Patch v0.3.13: on p2's own turn, p2's `legal_actions` lists no `activate` off p1's Machine
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
    /// the instance is followed by its id.
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
        let offered = s.prompt_modes();
        assert_eq!(offered.len(), 3);
        s.answer(json!(offered[0]));
        s.last_events().to_vec()
    }

    /// The abilities the card lists. The fused card may list its pick's own Activate (Classic #74's
    /// `plant`), a different ability from the Tree's, so "once a turn" is read on the Tree's own.
    fn abilities(s: &Scenario, id: &str) -> Vec<String> {
        listed(s, id).iter().filter_map(ability_of).collect()
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
        let own = abilities(&s, &id);
        assert!(!own.is_empty(), "the Tree lists its Activate");
        let events = grow(&mut s);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, GameEvent::Fused { .. }))
        );
        assert_ne!(grown(&s, &id), EVER_GROWING_TREE);
        assert!(!grown(&s, &id).contains('*'), "the base face fuses a base pick");
        assert!(
            abilities(&s, &id).iter().all(|ability| !own.contains(ability)),
            "Activate is once per turn"
        );
    }

    #[test]
    fn r384_radiant_activate_fuses_a_radiant_field_spell_it_offers_into_it_once_a_turn() {
        let (mut s, id) = planted(true);
        let own = abilities(&s, &id);
        assert!(!own.is_empty(), "the Tree lists its Activate");
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
        assert!(
            abilities(&s, &id).iter().all(|ability| !own.contains(ability)),
            "Activate is once per turn"
        );
    }
}

mod m_plus_n11_living_mana_potion {
    use super::*;

    /// The listed `activate`: p1 has Spell Damage +5 this turn (R1180001); the base face Tributes the
    /// potion, the Radiant face keeps it, once a turn.
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
    /// the Radiant face keeps it, once a turn.
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
    /// the Radiant face keeps it, once a turn.
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
/// `legal_actions`, shown usable on its controller's view and sent through `reduce` as listed, on both
/// faces.
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
        // R3142001: neither face asks anything; each Unit draws its own keywords.
        assert!(s.state().pending.is_none(), "nothing is asked");
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

    // R3044000: Twinspell's Activate grants its controller's next Spell this turn Echo +1, a rider
    // with no source; the base face then goes to the graveyard and the Radiant one stays.
    both_faces!(c079_twinspell, |radiant: bool| {
        let mut s = scenario(json!({
            "p1": { "backrow": [on(TWINSPELL, radiant)], "hand": [STOCKPILE], "library": [X, X] },
            "p2": { "hand": [FILLER], "library": [X, X] },
        }));
        let twin = s.card(TWINSPELL).id.clone();
        activate_listed(&mut s, TWINSPELL, any);
        let riders = s
            .state()
            .players
            .p1
            .mods
            .iter()
            .map(|m| serde_json::to_value(m).expect("a modifier serializes"))
            .filter(|m| m["kind"] == "echoNextSpell")
            .count();
        assert_eq!(riders, 1, "one Echo rider on its controller");
        if radiant {
            assert!(listed(&s, &twin).is_empty(), "once a turn");
        } else {
            s.expect_in_zone(&twin, "graveyard");
        }
    });

    both_faces!(s_n168_drone_hanger, |radiant: bool| {
        let mut s = scenario(json!({
            "p1": { "backrow": [on("stinkyscape-168", radiant)], "hand": [STOCKPILE], "library": [X, X] },
            "p2": { "hand": [FILLER], "library": [X, X] },
        }));
        let events = activate_listed(&mut s, "stinkyscape-168", any);
        assert_eq!(
            summoned_defs(&events),
            vec!["stinkyscape-t-drone-interceptor".to_string()]
        );
        let drone = s.unit(P1, 1).expect("the Interceptor in the leftmost unit zone");
        assert_eq!(drone.radiant, radiant);
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
        /// the engine gives it (no script declares it), at `STARSHIP_LAUNCH_COST`.
        #[test]
        fn r384_its_launch_through_the_listed_activate() {
            const HIT_JOB: &str = "core-016";
            const AP_ROUNDS: &str = "stinkyscape-100"; // AP Rounds: Starship Piece, Equip, Pierce
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
            assert_eq!(
                s.state().players.p1.mana.current,
                mana - jackioh_engine::prelude::STARSHIP_LAUNCH_COST
            );
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

    /// The Stinkyscape+ set's Activate cards, from its release (#3573): each ability is the action
    /// `legal_actions` lists, shown usable on its controller's view and sent through `reduce` as listed, on
    /// both faces (the cards' own tests drive them through the testkit's `activate`, which builds its own).
    mod plus {
        use super::*;

        const YATES: &str = "stinkyscapeplus-004";
        const ASSEMBLER_V3: &str = "stinkyscapeplus-008";
        const ASSEMBLER_V4: &str = "stinkyscapeplus-009";
        const ASSEMBLER_V5: &str = "stinkyscapeplus-010";
        const BOOSTER: &str = "stinkyscapeplus-011";
        const NORMALIZER: &str = "stinkyscapeplus-019";
        const VETERAN: &str = "stinkyscapeplus-029";
        const SCATTER: &str = "stinkyscapeplus-035";
        const ARC: &str = "stinkyscapeplus-038";
        const THORIUM: &str = "stinkyscapeplus-048";
        const REACTOR: &str = "stinkyscapeplus-055";
        const MASS_DRIVER: &str = "stinkyscapeplus-059";
        const LANCER: &str = "stinkyscapeplus-109";
        const SEI: &str = "stinkyscapeplus-124";
        const OMURA: &str = "stinkyscapeplus-125";
        const VELA: &str = "stinkyscapeplus-126";
        const CORVUS: &str = "stinkyscapeplus-127";
        const ASSEMBLY_LINE: &str = "stinkyscapeplus-136";
        const FIRE_BEAM_V3: &str = "stinkyscapeplus-143";
        const FIRE_BEAM_V4: &str = "stinkyscapeplus-144";
        const FIRE_BEAM_V5: &str = "stinkyscapeplus-145";
        const MORTAR_V3: &str = "stinkyscapeplus-146";
        const MORTAR_V4: &str = "stinkyscapeplus-147";
        const MORTAR_V5: &str = "stinkyscapeplus-148";
        const SHIPYARD: &str = "stinkyscapeplus-157";
        const ICARUS: &str = "stinkyscapeplus-160";

        /// What the abilities make: the drones, a Crawler, a Radi and a Warhero Token.
        const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
        const CORVETTE: &str = "stinkyscape-t-drone-corvette";
        const CRAWLER: &str = "stinkyscapeplus-120";
        const RADI: &str = "stinkyscapeplus-060";
        const WARHERO: &str = "stinkyscapeplus-t-warhero-token";

        /// What the boards are built of.
        const TOUGH: &str = "stand-in (1) 2/30"; // survives every hit and tick below
        const BODY: &str = "stand-in (1) 4/4 R12/12"; // a 4-health Unit: Trample's excess goes past it
        const TEXTED: &str = "stand-in (1) 3/5 Taunt, Rush"; // two keywords for Vanilla to strip
        const FLIER: &str = "stand-in (1) 2/30 Flying"; // Scatter's `damage2` lands on it
        const ARMORED: &str = "stand-in (0) 40/200 Armor 5"; // Pierce reads off its health
        const ARMORED_HEALTH: i32 = 200;
        const TYLANNT: &str = "stinkyscape-006"; // Tylannt: no M, O, U, S or E in its name, so Yates's exile spares it
        const FELINOR: &str = "core-086"; // "Miss" Mrow
        const KITTEN: &str = "meditative-030"; // Fickle E-Kitten: a second Felinor, so the discard is asked

        /// The turn does not end by itself when a use is spent, so the test reads the board it left.
        fn open(mut s: Scenario) -> Scenario {
            for player in [P1, P2] {
                s.state_mut().players[player].auto_end_turn = Some(false);
            }
            s
        }

        /// p1's `own` cards on the field against p2's `enemy` Units, each seat holding a spare card.
        fn facing(own: Value, enemy: Value) -> Scenario {
            open(scenario(json!({
                "p1": { "field": own, "hand": [STOCKPILE], "library": [X, X] },
                "p2": { "field": enemy, "hand": [FILLER], "library": [X, X] },
            })))
        }

        fn at_lane(def: &str, radiant: bool, lane: i32) -> Value {
            json!({ "def": def, "radiant": radiant, "lane": lane })
        }

        /// The listed `activate` that names `target` alone.
        fn aimed_at(target: &str) -> impl Fn(&ActionBody) -> bool + use<> {
            let target = instance(target);
            move |action: &ActionBody| targets_of(action) == vec![target.clone()]
        }

        /// "Activate N" (R384): the card has spent one of its `uses` a turn, so its view shows `uses - 1`
        /// left and it still lists an `activate` only when it had more than one.
        fn uses_left(s: &Scenario, id: &str, uses: i32) {
            assert_eq!(
                !listed(s, id).is_empty(),
                uses > 1,
                "{id}: one of {uses} uses a turn spent"
            );
            let shown: Vec<Value> = viewed(s, id)
                .iter()
                .map(|view| view["usesLeft"].clone())
                .collect();
            assert_eq!(shown, vec![json!(uses - 1)], "{id}: the uses its view shows left");
        }

        /// A token's printed attack on a face.
        fn token_attack(def: &str, radiant: bool) -> i32 {
            let card = &jackioh_cards::CATALOG[def];
            let face = if radiant { &card.radiant } else { &card.base };
            face.attack.unwrap_or_else(|| panic!("{def} prints an attack"))
        }

        /// R3537001: a Drone Assembler's Activate aimed at one enemy Unit summons its drone, which makes one
        /// forced attack on that Unit alone: the hit is the drone's own attack on its face.
        fn assembles(def: &str, drone: &str, uses: i32, radiant: bool) {
            let mut s = facing(json!([on(def, radiant)]), json!([TOUGH, TOUGH]));
            let id = s.card(def).id.clone();
            let (other, target) = (s.unit_at(P2, 1).id, s.unit_at(P2, 2).id);
            let events = activate_listed(&mut s, def, aimed_at(&target));
            assert_eq!(summoned_defs(&events), vec![drone.to_string()], "{def}");
            let hits = hits_on(&events, &target);
            assert_eq!(
                hits.first().copied(),
                Some(token_attack(drone, radiant)),
                "{def}: {hits:?}"
            );
            assert!(
                hits_on(&events, &other).is_empty(),
                "{def}: the other Unit is not struck"
            );
            uses_left(&s, &id, uses);
        }

        both_faces!(sp_n8_drone_assembler_v3, |radiant: bool| assembles(
            ASSEMBLER_V3,
            INTERCEPTOR,
            1,
            radiant
        ));
        both_faces!(sp_n9_drone_assembler_v4, |radiant: bool| assembles(
            ASSEMBLER_V4,
            INTERCEPTOR,
            2,
            radiant
        ));
        both_faces!(sp_n10_drone_assembler_v5, |radiant: bool| assembles(
            ASSEMBLER_V5,
            CORVETTE,
            1,
            radiant
        ));

        // R3548003: Refresh gives back this turn's uses of every other Activate, and is never given back
        // itself: 1 use on the base face, 2 on the Radiant one.
        both_faces!(sp_n11_energy_booster, |radiant: bool| {
            let mut s = facing(json!([on(BOOSTER, radiant), ASSEMBLER_V3]), json!([TOUGH, TOUGH]));
            let (booster, assembler) = (s.card(BOOSTER).id.clone(), s.card(ASSEMBLER_V3).id.clone());
            activate_listed(&mut s, ASSEMBLER_V3, any);
            assert!(
                listed(&s, &assembler).is_empty(),
                "the Assembler's one use is spent"
            );
            activate_listed(&mut s, BOOSTER, any);
            assert!(
                !listed(&s, &assembler).is_empty(),
                "Refresh gave the Assembler's use back"
            );
            uses_left(&s, &booster, if radiant { 2 } else { 1 });
        });

        // R3552002: Vanilla strips the named Unit's keywords; 1 use on the base face, 2 on the Radiant one.
        both_faces!(sp_n19_normalization_module, |radiant: bool| {
            let mut s = facing(json!([on(NORMALIZER, radiant)]), json!([TEXTED]));
            let (module, enemy) = (s.card(NORMALIZER).id.clone(), s.unit_at(P2, 1).id);
            assert_eq!(s.stats(&enemy).keywords.len(), 2, "Taunt and Rush to lose");
            activate_listed(&mut s, NORMALIZER, aimed_at(&enemy));
            assert!(s.stats(&enemy).keywords.is_empty(), "Vanilla strips its keywords");
            uses_left(&s, &module, if radiant { 2 } else { 1 });
        });

        // R3553001: the Felinor to discard is the controller's pick, asked as the Activate resolves; each
        // Warhero Token it adds wears the card's face.
        both_faces!(sp_n29_grinder_veteran, |radiant: bool| {
            let mut s = open(scenario(json!({
                "p1": { "field": [on(VETERAN, radiant)], "hand": [FELINOR, KITTEN, STOCKPILE], "library": [X, X] },
                "p2": { "hand": [FILLER], "library": [X, X] },
            })));
            let (id, discarded) = (s.card(VETERAN).id.clone(), s.card(KITTEN).id.clone());
            activate_listed(&mut s, VETERAN, any);
            let pending = s.state().pending.clone().expect("a hand prompt is open");
            assert_eq!((pending.kind, pending.player_id), (PromptKind::Hand, P1));
            s.answer(json!(discarded));
            let graveyard = s.pile(P1, "graveyard");
            assert!(
                graveyard.iter().any(|card| card.id == discarded),
                "the chosen Felinor is discarded"
            );
            let tokens: Vec<CardInstance> = s
                .hand(P1)
                .into_iter()
                .filter(|card| card.def_id == WARHERO)
                .collect();
            assert_eq!(tokens.len() as i32, printed(VETERAN, "summons", radiant));
            assert!(
                tokens.iter().all(|card| card.radiant == radiant),
                "Warhero Tokens of its face"
            );
            uses_left(&s, &id, 1);
        });

        // R3559000: every enemy Unit, then the enemy hero, rolls its own amount in the face's range (0~2,
        // Radiant 1~3; a 0 is no hit), and a Flying Unit takes `damage2` instead; 1 use, Radiant 2.
        both_faces!(sp_n35_scatter, |radiant: bool| {
            let mut s = facing(json!([on(SCATTER, radiant)]), json!([TOUGH, FLIER]));
            let (id, ground, flier) = (
                s.card(SCATTER).id.clone(),
                s.unit_at(P2, 1).id,
                s.unit_at(P2, 2).id,
            );
            let events = activate_listed(&mut s, SCATTER, any);
            let range = if radiant { 1..=3 } else { 0..=2 };
            for target in [ground.as_str(), "hero-p2"] {
                let hit = hits_on(&events, target);
                assert!(
                    hit.len() <= 1 && hit.iter().all(|amount| range.contains(amount)),
                    "{target}: {hit:?}"
                );
                assert!(
                    range.contains(&0) || !hit.is_empty(),
                    "{target}: a roll of at least 1 hits"
                );
            }
            assert_eq!(
                hits_on(&events, &flier),
                vec![printed(SCATTER, "damage2", radiant)],
                "the flier"
            );
            uses_left(&s, &id, if radiant { 2 } else { 1 });
        });

        // R3559001, R3554006: the Unit named takes `damage2`, then the chain hits each Unit beside a hit one
        // once; its label's uses a turn, "Activate 2:" and on the Radiant face "Activate 5:" (R482).
        both_faces!(sp_n38_arc, |radiant: bool| {
            let mut s = facing(
                json!([at_lane(ARC, radiant, 5)]),
                json!([
                    at_lane(TOUGH, false, 1),
                    at_lane(TOUGH, false, 2),
                    at_lane(TOUGH, false, 4)
                ]),
            );
            let id = s.card(ARC).id.clone();
            let lanes = [1, 2, 4].map(|lane| s.unit_at(P2, lane).id);
            let events = activate_listed(&mut s, ARC, aimed_at(&lanes[0]));
            let dealt = printed(ARC, "damage2", radiant);
            assert_eq!(hits_on(&events, &lanes[0]), vec![dealt], "the Unit named");
            assert_eq!(
                hits_on(&events, &lanes[1]),
                vec![dealt],
                "chained to the Unit beside it"
            );
            assert!(
                hits_on(&events, &lanes[2]).is_empty(),
                "lane 4 is beside no Unit hit"
            );
            uses_left(&s, &id, if radiant { 5 } else { 2 });
        });

        // R3556005: the Activate discards a chosen Thorium from hand to skip this turn's blow-up roll; with
        // no Thorium in hand it is not offered.
        both_faces!(sp_n55_thorium_reactor, |radiant: bool| {
            let reactor = |hand: Value| {
                open(scenario(json!({
                    "p1": { "backrow": [on(REACTOR, radiant)], "hand": hand, "library": [X, X] },
                    "p2": { "hand": [FILLER], "library": [X, X] },
                })))
            };
            let bare = reactor(json!([STOCKPILE]));
            assert!(
                listed(&bare, &bare.card(REACTOR).id).is_empty(),
                "no Thorium, no Activate"
            );
            let mut s = reactor(json!([THORIUM, STOCKPILE]));
            let (id, thorium) = (s.card(REACTOR).id.clone(), s.card(THORIUM).id.clone());
            activate_listed(&mut s, REACTOR, any);
            if s.state().pending.is_some() {
                s.answer(json!(thorium));
            }
            assert!(
                s.pile(P1, "graveyard").iter().any(|card| card.id == thorium),
                "the Thorium is discarded"
            );
            uses_left(&s, &id, 1);
        });

        // R3556007: the Activate queues a delivery: at the start of its controller's `count`th own turn, two
        // Minerals of its face at (0) (and a Resource).
        both_faces!(sp_n59_mass_driver, |radiant: bool| {
            let mut s = open(scenario(json!({
                "p1": { "backrow": [on(MASS_DRIVER, radiant)], "hand": [STOCKPILE], "library": [X, X, X] },
                "p2": { "hand": [FILLER], "library": [X, X, X] },
            })));
            let id = s.card(MASS_DRIVER).id.clone();
            activate_listed(&mut s, MASS_DRIVER, any);
            uses_left(&s, &id, 1);
            for _ in 0..printed(MASS_DRIVER, "count", radiant) {
                s.end_turn().end_turn();
            }
            let minerals: Vec<CardInstance> = s
                .hand(P1)
                .into_iter()
                .filter(|card| {
                    jackioh_cards::CATALOG[card.def_id.as_str()]
                        .tags
                        .contains(&Tag::Mineral)
                })
                .collect();
            assert_eq!(minerals.len(), 2, "two Minerals delivered");
            for mineral in &minerals {
                assert_eq!(mineral.radiant, radiant, "of the Driver's face");
                assert_eq!(
                    effective_cost(s.state(), mineral, Default::default()),
                    0,
                    "at (0)"
                );
            }
        });

        // R3560008: Pierce Cleave: the Unit aimed at and each Unit beside it take the hit, Armor and all.
        both_faces!(sp_n124_sei, |radiant: bool| {
            let mut s = facing(json!([on(SEI, radiant)]), json!([TOUGH, ARMORED, TOUGH, TOUGH]));
            let id = s.card(SEI).id.clone();
            let lanes = [1, 2, 3, 4].map(|lane| s.unit_at(P2, lane).id);
            let events = activate_listed(&mut s, SEI, aimed_at(&lanes[1]));
            let dealt = printed(SEI, "damage", radiant);
            for lane in &lanes[..3] {
                assert_eq!(hits_on(&events, lane), vec![dealt], "{lane}");
            }
            assert!(
                hits_on(&events, &lanes[3]).is_empty(),
                "the fourth Unit is out of reach"
            );
            assert_eq!(s.stats(&lanes[1]).health, ARMORED_HEALTH - dealt, "Pierce");
            uses_left(&s, &id, if radiant { 2 } else { 1 });
        });

        // R3560008, R3550003: Pierce Trample: what the Unit cannot bear goes on to its controller's hero; the
        // use is spent and, with its 3-turn cooldown, listed again no sooner than the third turn.
        both_faces!(sp_n125_omura, |radiant: bool| {
            let mut s = facing(json!([on(OMURA, radiant)]), json!([BODY]));
            let (id, body) = (s.card(OMURA).id.clone(), s.unit_at(P2, 1).id);
            let (dealt, bears) = (printed(OMURA, "damage", radiant), s.stats(&body).health);
            let events = activate_listed(&mut s, OMURA, aimed_at(&body));
            assert_eq!(hits_on(&events, &body), vec![bears]);
            assert_eq!(hits_on(&events, "hero-p2"), vec![dealt - bears]);
            assert_eq!(health(&s, P2), HERO_HEALTH - (dealt - bears));
            assert!(listed(&s, &id).is_empty(), "the use is spent");
            let sits_out: Vec<Value> = viewed(&s, &id)
                .iter()
                .map(|view| view["cooldownTurnsLeft"].clone())
                .collect();
            assert_eq!(sits_out, vec![json!(2)], "a 3-turn cooldown sits out two turns");
        });

        /// R3554005: Vela, Corvus and Lancer ask for an enemy tile and hit every Unit on the line to it with
        /// Pierce; `cleaves`, each Unit beside the one hit too (Vela's and Corvus's Radiant face). `uses` a turn.
        fn draws_a_line(def: &str, radiant: bool, cleaves: bool, uses: i32) {
            let mut s = facing(
                json!([at_lane(def, radiant, 1)]),
                json!([
                    at_lane(ARMORED, false, 1),
                    at_lane(ARMORED, false, 2),
                    at_lane(ARMORED, false, 3)
                ]),
            );
            let id = s.card(def).id.clone();
            let lanes = [1, 2, 3].map(|lane| s.unit_at(P2, lane).id);
            activate_listed(&mut s, def, any);
            // The tile at the enemy Units in lane 1, as the card's own tests aim at it: forced into the
            // offer when the offer's own cells name it otherwise.
            let wanted = "0,2".to_string();
            let pending = s.state_mut().pending.as_mut().expect("a tile prompt is open");
            if !pending.options.iter().any(|option| option.key == wanted) {
                pending.options[0] = PromptOption {
                    key: wanted.clone(),
                    label: "Lane 1, forced".to_string(),
                    selection: Selection::Mode {
                        option: wanted.clone(),
                    },
                    cost: None,
                    radiant: None,
                };
            }
            s.answer(json!([wanted]));
            let events = s.last_events().to_vec();
            let dealt = printed(def, "damage", radiant);
            assert_eq!(hits_on(&events, &lanes[0]), vec![dealt], "{def}");
            assert_eq!(s.stats(&lanes[0]).health, ARMORED_HEALTH - dealt, "{def}: Pierce");
            let beside = if cleaves { vec![dealt] } else { vec![] };
            assert_eq!(
                hits_on(&events, &lanes[1]),
                beside,
                "{def}: Cleave on the Radiant face only"
            );
            assert!(hits_on(&events, &lanes[2]).is_empty(), "{def}: two lanes away");
            uses_left(&s, &id, uses);
        }

        both_faces!(sp_n126_vela, |radiant: bool| draws_a_line(
            VELA, radiant, radiant, 1
        ));
        both_faces!(sp_n127_corvus, |radiant: bool| draws_a_line(
            CORVUS, radiant, radiant, 1
        ));
        // R3554005: Lancer's line never Cleaves; 1 use on the base face, 2 on the Radiant one.
        both_faces!(sp_n109_lancer, |radiant: bool| draws_a_line(
            LANCER,
            radiant,
            false,
            if radiant { 2 } else { 1 }
        ));

        // R3560000: the Assembly Line, a Field Spell, summons a Crawler of its face into the leftmost zone.
        both_faces!(sp_n136_crawler_assembly_line, |radiant: bool| {
            let mut s = open(scenario(json!({
                "p1": { "backrow": [on(ASSEMBLY_LINE, radiant)], "hand": [STOCKPILE], "library": [X, X] },
                "p2": { "hand": [FILLER], "library": [X, X] },
            })));
            let id = s.card(ASSEMBLY_LINE).id.clone();
            let events = activate_listed(&mut s, ASSEMBLY_LINE, any);
            assert_eq!(summoned_defs(&events), vec![CRAWLER.to_string()]);
            let crawler = s
                .unit(P1, 1)
                .expect("the Crawler stands in the leftmost unit zone");
            assert_eq!((crawler.def_id.as_str(), crawler.radiant), (CRAWLER, radiant));
            uses_left(&s, &id, 1);
        });

        /// R3544001: a Fire Beam puts its Burn on the Unit aimed at and each Unit beside it, no other, and
        /// deals no damage.
        fn burns(def: &str, uses: i32, radiant: bool) {
            let mut s = facing(json!([on(def, radiant)]), json!([TOUGH, TOUGH, TOUGH, TOUGH]));
            let id = s.card(def).id.clone();
            let lanes = [1, 2, 3, 4].map(|lane| s.unit_at(P2, lane).id);
            let events = activate_listed(&mut s, def, aimed_at(&lanes[1]));
            let burn = printed(def, "burn", radiant);
            let burning: Vec<i32> = lanes
                .iter()
                .map(|lane| jackioh_engine::query::burn_on(s.card(lane)))
                .collect();
            assert_eq!(burning, vec![burn, burn, burn, 0], "{def}");
            assert!(
                lanes.iter().all(|lane| hits_on(&events, lane).is_empty()),
                "{def}: no damage"
            );
            uses_left(&s, &id, uses);
        }

        both_faces!(sp_n143_fire_beam_v3, |radiant: bool| burns(
            FIRE_BEAM_V3,
            1,
            radiant
        ));
        both_faces!(sp_n144_fire_beam_v4, |radiant: bool| burns(
            FIRE_BEAM_V4,
            1,
            radiant
        ));
        both_faces!(sp_n145_fire_beam_v5, |radiant: bool| burns(
            FIRE_BEAM_V5,
            2,
            radiant
        ));

        /// R3544000: a Scourge Mortar puts its Toxic on the Unit aimed at alone and deals no damage; 1 use on
        /// the base face, 2 on the Radiant one.
        fn toxes(def: &str, radiant: bool) {
            let mut s = facing(json!([on(def, radiant)]), json!([TOUGH, TOUGH]));
            let id = s.card(def).id.clone();
            let lanes = [1, 2].map(|lane| s.unit_at(P2, lane).id);
            let events = activate_listed(&mut s, def, aimed_at(&lanes[1]));
            let toxic = |lane: &String| jackioh_engine::query::toxic_on(s.card(lane));
            assert_eq!(toxic(&lanes[1]), printed(def, "toxic", radiant), "{def}");
            assert_eq!(toxic(&lanes[0]), 0, "{def}: the Unit beside it is not touched");
            assert!(
                hits_on(&events, &lanes[1]).is_empty(),
                "{def}: Toxic is no damage"
            );
            uses_left(&s, &id, if radiant { 2 } else { 1 });
        }

        both_faces!(sp_n146_scourge_mortar_v3, |radiant: bool| toxes(
            MORTAR_V3, radiant
        ));
        both_faces!(sp_n147_scourge_mortar_v4, |radiant: bool| toxes(
            MORTAR_V4, radiant
        ));
        both_faces!(sp_n148_scourge_mortar_v5, |radiant: bool| toxes(
            MORTAR_V5, radiant
        ));

        /// R3539004: a Starship's pieces as (card, whether Radiant).
        fn pieces_of(s: &Scenario, id: &str) -> Vec<(String, bool)> {
            jackioh_engine::prelude::starship_pieces_of(s.card(id))
                .into_iter()
                .map(|piece| (piece.def_id, piece.radiant == Some(true)))
                .collect()
        }

        // R3539004: the shipyard fuses a piece at the start of its turn (a Radiant one on the Radiant face);
        // its Activate exiles it and launches a Starship holding that piece.
        both_faces!(sp_n157_phantom_shipyard, |radiant: bool| {
            let mut s = open(scenario(json!({
                "seed": "listed-s157",
                "p1": { "hand": [on(SHIPYARD, radiant)], "library": [X, X, X, X], "mana": 10 },
                "p2": { "hand": [FILLER], "library": [X, X, X, X] },
            })));
            s.play(SHIPYARD, json!({ "zone": 2 }));
            let yard = s.unit(P1, 2).expect("the shipyard fills lane 2").id;
            s.end_turn().end_turn();
            let held = pieces_of(&s, &yard);
            assert_eq!(held.len(), 1, "a piece fused in at the start of its turn");
            assert_eq!(held[0].1, radiant, "a Radiant piece on the Radiant face only");
            activate_listed(&mut s, &yard, any);
            s.expect_in_zone(yard.as_str(), "exile");
            let ship = jackioh_engine::prelude::launched_starship_of(s.state(), P1)
                .cloned()
                .expect("a Starship launched");
            assert_eq!(
                pieces_of(&s, &ship.id),
                held,
                "the Starship holds the shipyard's piece"
            );
        });

        // R411: the Radi is summoned and its Cry triggered: a Discover of Weapons, Rare ones from a base Radi
        // and Epic ones from a Radiant Radi, the one picked equipped into it.
        both_faces!(sp_n160_icarus, |radiant: bool| {
            let mut s = facing(json!([on(ICARUS, radiant)]), json!([TOUGH]));
            let id = s.card(ICARUS).id.clone();
            let events = activate_listed(&mut s, ICARUS, any);
            assert_eq!(summoned_defs(&events), vec![RADI.to_string()]);
            let radi = s.unit_at(P1, 2);
            assert_eq!((radi.def_id.as_str(), radi.radiant), (RADI, radiant));
            let pending = s.state().pending.clone().expect("the Radi's Discover is open");
            assert_eq!((pending.kind, pending.player_id), (PromptKind::Discover, P1));
            let offered = s.prompt_modes();
            let rarity = if radiant { Rarity::Epic } else { Rarity::Rare };
            assert!(!offered.is_empty(), "weapons are offered");
            for def in &offered {
                let card = &jackioh_cards::CATALOG[def.as_str()];
                assert!(card.tags.contains(&Tag::Weapon) && card.rarity == rarity, "{def}");
            }
            let before = s.stats(&radi.id).health;
            s.answer(json!(offered[0]));
            let weapon = &jackioh_cards::CATALOG[offered[0].as_str()];
            let worn = if radiant { &weapon.radiant } else { &weapon.base };
            assert_eq!(s.stats(&radi.id).health, before + worn.health.unwrap_or_default());
            uses_left(&s, &id, 1);
        });

        /// S+ #4 Profe Yates in p1's hand with only itself to lose to its Cry's exile, p1's `own` Units and
        /// p2's `enemy` Units on the field (Tylannt: no M, O, U, S or E in its name).
        fn yates_in_hand(radiant: bool, own: Value, enemy: Value) -> Scenario {
            open(scenario(json!({
                "seed": "listed-s004",
                "p1": { "mana": 10, "hand": [on(YATES, radiant)], "field": own, "library": [X, X] },
                "p2": { "field": enemy, "hand": [FILLER], "library": [X, X] },
            })))
        }

        // R3567003: Overwhelm freezes every Unit in its lane and the lanes beside it, both sides, itself too.
        both_faces!(sp_n4_profe_yates_overwhelm, |radiant: bool| {
            let own = json!([at_lane(TYLANNT, false, 1), at_lane(TYLANNT, false, 4)]);
            let enemy = json!([at_lane(TYLANNT, false, 2), at_lane(TYLANNT, false, 5)]);
            let mut s = yates_in_hand(radiant, own, enemy);
            s.play(YATES, json!({ "zone": 2, "modes": ["overwhelm"] }));
            let id = s.card(YATES).id.clone();
            activate_listed(&mut s, YATES, any);
            let frozen = |player: PlayerId, lane: i32| s.unit_at(player, lane).frozen.is_some();
            assert_eq!(
                [
                    frozen(P1, 1),
                    frozen(P1, 2),
                    frozen(P1, 4),
                    frozen(P2, 2),
                    frozen(P2, 5)
                ],
                [true, true, false, true, false]
            );
            uses_left(&s, &id, 1);
        });

        // R3567004: Wolf's Maw pulls a Flying Unit down: it loses Flying and takes the declared damage.
        both_faces!(sp_n4_profe_yates_wolfs_maw, |radiant: bool| {
            let mut s = yates_in_hand(radiant, json!([]), json!([on(TYLANNT, true)]));
            let flyer = s.unit_at(P2, 1).id;
            find_instance_mut(s.state_mut(), &flyer)
                .expect("the enemy Unit stands")
                .granted_keywords
                .push(Keyword::Flying);
            s.play(YATES, json!({ "zone": 1, "modes": ["wolfsMaw"] }));
            activate_listed(&mut s, YATES, aimed_at(&flyer));
            let pulled = s.card(&flyer);
            assert!(!jackioh_engine::layers::unit_has(
                s.state(),
                pulled,
                KeywordKind::Flying
            ));
            assert_eq!(pulled.damage, printed(YATES, "damage", radiant));
        });

        // R3567002: Iron Mandate fuses Yates into a friendly Unit, kept: the host takes its stats.
        both_faces!(sp_n4_profe_yates_iron_mandate, |radiant: bool| {
            let mut s = yates_in_hand(radiant, json!([at_lane(TYLANNT, radiant, 2)]), json!([]));
            s.play(YATES, json!({ "zone": 1, "modes": ["ironMandate"] }));
            let (yates, host) = (s.unit_at(P1, 1).id, s.unit_at(P1, 2).id);
            let (own, grafted) = (s.stats(&host).attack, s.stats(&yates).attack);
            activate_listed(&mut s, YATES, aimed_at(&host));
            s.expect_in_zone(yates.as_str(), "gone");
            s.expect_stats(host.as_str(), json!({ "attack": own + grafted }));
        });
    }
}
