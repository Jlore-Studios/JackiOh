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
const BROTHER_PING: &str = "classicplus-076-1";
const GACHAHOLIC: &str = "meditative-101";

/// Every card or token whose base or Radiant text has an Activate ability, each with a case below.
const ACTIVATE_CARDS: [&str; 15] = [
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
    BROTHER_PING,
    GACHAHOLIC,
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

    fn plants(radiant: bool, tokens: i32) {
        let mut s = scenario(json!({
            "p1": { "backrow": [on(CORPSE_PLANTATION, radiant)], "hand": [STOCKPILE] },
            "p2": { "hand": [FILLER] },
        }));
        activate_listed(&mut s, CORPSE_PLANTATION, any);
        assert_eq!(s.card(CORPSE_PLANTATION).counters.plague, Some(tokens));
    }

    #[test]
    fn r384_base_places_2_plague_counters_on_itself() {
        plants(false, 2);
    }

    #[test]
    fn r384_radiant_places_4_plague_counters_on_itself() {
        plants(true, 4);
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
        // Activate ♾️: listed again while a permanent has a counter.
        activate_listed(&mut s, MUTATE_SPELL, at(enemy.clone()));
        s.expect_in_zone(&enemy, "exile");
    }

    #[test]
    fn r384_radiant_a_counter_off_your_backrow_card_draws_4() {
        let mut s = mutate(true);
        let well = s.card(MANA_WELL).id.clone();
        let hand = s.hand(P1).len();
        activate_listed(&mut s, MUTATE_SPELL, at(well));
        assert_eq!(s.hand(P1).len(), hand + 4);
    }
}

mod c_n81_the_power_to_thrive {
    use super::*;

    fn gains(radiant: bool, mana: i32) {
        let mut s = scenario(json!({
            "p1": { "backrow": [on(POWER_TO_THRIVE, radiant)], "hand": [STOCKPILE], "mana": 3 },
            "p2": { "hand": [FILLER] },
        }));
        activate_listed(&mut s, POWER_TO_THRIVE, |action| {
            modes_of(action) == vec!["mana".to_string()]
        });
        s.expect_mana(P1, 3 + mana);
        let id = s.card(POWER_TO_THRIVE).id.clone();
        assert!(listed(&s, &id).is_empty(), "once per turn");
    }

    #[test]
    fn r384_base_the_mana_mode_gains_1() {
        gains(false, 1);
    }

    #[test]
    fn r384_radiant_the_mana_mode_gains_2() {
        gains(true, 2);
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
