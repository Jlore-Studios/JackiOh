//! Every Activate ability, used the way a client uses it (#491, SPEC §6.2, R384, R752): the
//! `activate` is found in `legal_actions`, the controller's own view lists the ability as usable,
//! and the very action listed goes through `reduce` and does what the card's text says, on the base
//! face and on the Radiant one.
//!
//! The per-card tests mostly drive an ability through the testkit's `activate` verb, which builds its
//! own action (the `activatePower` alias when it names no ability, R752). A client never builds one:
//! it sends back an action `legal_actions` listed, and online that action is sent over the socket
//! (`crates/server/tests/actor/match_actor.rs`, `r384_activate_over_the_socket`). This file is the
//! engine half of that path for every Activate card at once, and `activate_census` keeps it whole:
//! a card whose text gains an Activate must gain a case here.

use jackioh_engine::config::LIFE_TAP_DAMAGE;
use jackioh_engine::testkit::*;

use super::scenario;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

const HEROIC_POWER: &str = "core-098";
const INFINISCEPTER: &str = "classic-007";
const NOSE_HUNTER: &str = "classic-015";
const POWER_TO_PUNISH: &str = "classic-020";
const TURTINATOR: &str = "classic-021";
const DEVILS_PACT: &str = "classic-023";
const TRANSMUTABLE_TOXINS: &str = "classic-042";
const CORPSE_PLANTATION: &str = "classic-074";
const MUTATE_SPELL: &str = "classic-078";
const POWER_TO_THRIVE: &str = "classic-081";
const LOCKDOWN: &str = "classic-084";
const FUSION_LAB: &str = "classicplus-031";
const DOCTORS_ORDERS: &str = "classicplus-060";
const KYS_GIFT: &str = "classicplus-042-1";
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

/// Every card or token whose base or Radiant text has an Activate ability, each with a case below.
const ACTIVATE_CARDS: [&str; 26] = [
    HEROIC_POWER,
    INFINISCEPTER,
    NOSE_HUNTER,
    POWER_TO_PUNISH,
    TURTINATOR,
    DEVILS_PACT,
    TRANSMUTABLE_TOXINS,
    CORPSE_PLANTATION,
    MUTATE_SPELL,
    POWER_TO_THRIVE,
    LOCKDOWN,
    FUSION_LAB,
    DOCTORS_ORDERS,
    KYS_GIFT,
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
];

const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2. A spare that keeps R82's auto-end away.
const MANA_WELL: &str = "core-006"; // (3) Field Spell: a backrow card of p1's to hold a Plague Counter.
const VANILLA: &str = "core-008"; // (1) Unit 4/4.
const FILLER: &str = "core-010"; // (0) Spell.
const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
const X: &str = "core-020"; // library filler.
const CONJURE_KY: &str = "core-057"; // (2) Spell: Add 3 random KY cards to your hand.
const APPLE: &str = "classicplus-059"; // All Purpose Apple, Doctors Orders' card.

/// Writes JSON for a field entry on the face asked for.
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

/// The `activate` actions `legal_actions` lists for p1 on that card, in its order.
fn listed(s: &Scenario, card: &str) -> Vec<ActionBody> {
    legal_actions(s.state(), P1)
        .into_iter()
        .filter(|action| matches!(action, ActionBody::Activate { instance_id, .. } if instance_id == card))
        .collect()
}

fn targets_of(action: &ActionBody) -> Vec<Selection> {
    match action {
        ActionBody::Activate { targets, .. } => targets.clone().unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn modes_of(action: &ActionBody) -> Vec<String> {
    match action {
        ActionBody::Activate { modes, .. } => modes.clone().unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn tributes_of(action: &ActionBody) -> Vec<String> {
    match action {
        ActionBody::Activate { tributes, .. } => tributes.clone().unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn ability_of(action: &ActionBody) -> Option<String> {
    match action {
        ActionBody::Activate { ability, .. } => ability.clone(),
        _ => None,
    }
}

/// The first object in `value` naming `card` that carries `activations`: the card on p1's own view.
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
    let view = serde_json::to_value(s.view(P1)).expect("PlayerView serialises");
    activations_in(&view["you"], card).unwrap_or_default()
}

/// Uses the card's ability the way a client does: the `activate` `legal_actions` lists that `pick`
/// accepts, which p1's view shows as usable, sent through `reduce` as it was listed. Answers the
/// events it produced (`activated` among them) and leaves the scenario on its result.
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

/// The census: the cards this file covers are exactly the cards whose text has an Activate, which are
/// exactly the cards whose script declares one.
#[test]
fn r384_activate_census_every_card_whose_text_has_an_activate_declares_one_and_has_a_case_here() {
    jackioh_cards::register_all();
    let scripts = jackioh_cards::scripts_of();
    let mut by_text: Vec<String> = Vec::new();
    let mut by_script: Vec<String> = Vec::new();
    for (id, def) in jackioh_cards::CATALOG.iter() {
        if def.base.text.contains("Activate") || def.radiant.text.contains("Activate") {
            by_text.push(id.clone());
        }
        if scripts.get(id).is_some_and(|script| {
            !script.base.activations.is_empty() || !script.radiant.activations.is_empty()
        }) {
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
    /// writes it), a spare in hand and a library each.
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

    /// Plays the InfiniScepter, its Cry exiling `spell` from p1's hand.
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
        // Stockpile's copy: Draw 2. Heal your hero 2.
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
        // Activate ♾️: listed again while a card is left to discard.
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

    /// After the activation, Mr. Vanilla played is a Book of Flame, aimed at p2's hero.
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
        s.expect_health(P2, 26);
    }

    #[test]
    fn r449_radiant_each_card_played_this_turn_is_a_radiant_book_of_flame() {
        let mut s = pact(true);
        activate_listed(&mut s, DEVILS_PACT, any);
        play_vanilla_as_a_book(&mut s);
        s.expect_health(P2, 22);
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

    /// The one Unit on the field takes the Plague Counters, and the Aura grows it by `per` each.
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

mod c_n74_corpse_plantation {
    use super::*;

    /// The Activate arms one play from the graveyard this turn (patch v0.3.10; its Cry places the
    /// counters now), remembering the turn it was armed on.
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
        // Activate, once a turn (patch v0.3.10): not listed again, though the enemy has a counter.
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
        // Activate ♾️: listed again while a permanent has a counter.
        activate_listed(&mut s, MUTATE_SPELL, at(enemy.clone()));
        s.expect_in_zone(&enemy, "exile");
    }
}

mod c_n81_the_power_to_thrive {
    use super::*;

    /// The third mode, a card of your hand costing less (patch v0.3.10; it gained mana before).
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

mod c_n31_fusion_lab {
    use super::*;

    /// Fuses a random card into the Midrange Menace the listed activation names; answers the fused
    /// card's ingredient specs, the random card first.
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

/// The `upgraded` events in `events` naming `card`.
fn upgrades_of(events: &[GameEvent], card: &str) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, GameEvent::Upgraded { instance_id, .. } if instance_id == card))
        .count()
}

mod m_n97_2_wishing_well {
    use super::*;

    /// The Well at 100%, so the roll cannot miss.
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

    /// The University beside one Vanilla: it Buffs its other permanents, never itself (patch v0.3.10).
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
    fn r384_radiant_activate_buffs_the_other_permanent_five_times_once_a_turn() {
        let mut s = campus(true);
        let id = s.card(UNIVERSITY).id.clone();
        let other = s.card(VANILLA).id.clone();
        let events = activate_listed(&mut s, UNIVERSITY, any);
        assert_eq!(upgrades_of(&events, &other), 5);
        assert_eq!(upgrades_of(&events, &id), 0);
        assert!(listed(&s, &id).is_empty(), "Activate is once per turn");
    }
}

mod m_n97_9_jlockheeds_headquarters {
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

    /// Its controller uses the listed `activate`: 2 of p1's mana, and the Machine is Tributed.
    fn tributes_itself(radiant: bool) {
        let mut s = scenario(json!({
            "p1": { "backrow": [on(MONEY_MACHINE, radiant)], "hand": [STOCKPILE] },
            "p2": { "hand": [FILLER] },
        }));
        let mana = s.state().players.p1.mana.current;
        activate_listed(&mut s, MONEY_MACHINE, any);
        assert_eq!(s.state().players.p1.mana.current, mana - 2);
        s.expect_in_zone(MONEY_MACHINE, "graveyard");
    }

    #[test]
    fn r384_r1105_base_spends_2_and_tributes_itself() {
        tributes_itself(false);
    }

    #[test]
    fn r384_r1105_radiant_spends_2_and_tributes_itself() {
        tributes_itself(true);
    }

    /// MD-D9: on p2's own turn, p2's `legal_actions` lists the `activate` off p1's Machine too.
    #[test]
    fn r1105_the_opponent_is_listed_it_on_their_own_turn() {
        let s = scenario(json!({
            "active": "p2",
            "p1": { "backrow": [MONEY_MACHINE], "hand": [STOCKPILE] },
            "p2": { "hand": [FILLER] },
        }));
        let id = s.card(MONEY_MACHINE).id.clone();
        assert!(
            legal_actions(s.state(), P2).into_iter().any(
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

    /// p1's Maestro with `counters` Plague Counters; p2 holds cards in every zone the Activate
    /// reaches: two on the field, three in hand, three in the deck and two in the graveyard.
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

    /// p2's cards in each zone the Activate reaches: field, hand, deck, graveyard.
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

    /// Activates the Tree as `legalActions` lists it and takes the first of the two Field Spells it
    /// offers (patch v0.3.10); the events of the pick.
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
        assert_eq!(offered.len(), 2);
        s.answer(json!(offered[0]));
        s.last_events().to_vec()
    }

    /// The Tree's def id now: a fusion of a picked Field Spell and the Tree.
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
