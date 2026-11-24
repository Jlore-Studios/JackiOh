//! Audit of the engine's combat and damage (`combat.rs`, `damage.rs`, `subsystems/lethal.rs`,
//! `query::combat_forecast`). Each test asserts the ruled behaviour and FAILS on the code as it stands,
//! but the one named `control_…`, which passes and pins the baseline.
//!
//! One root cause, three cases: R44's lethal projection (`subsystems::lethal::is_lethal`) nets the heal
//! of a defender's Lifesteal strike back (R176) through `strike_back_heal`, which returns the strike
//! back's plain damage: no healing multiplier (R1180003), no lane strike (R1283), and no conversion of
//! the heal into damage (R413). My Pawn (#96), its glow, the combat forecast's `overflow.kills` and the
//! AI's reply model all read it.

use jackioh_engine::testkit::*;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

const VANILLA: &str = "core-008"; // Mr. Vanilla, 4/4, no text
const LIFE_MAGE: &str = "meditativeplus-007"; // Radiant: "healing your hero and Units is twice as effective"
const MY_PAWN: &str = "core-096"; // Trap: "When the opponent declares an attack that would be lethal to your hero: cancel it, ..."

/// p1's 10/10 Trample attacker faces p2's 3/2 Lifesteal defender; p2's hero has 5 life and no Armor.
/// The blow deals the defender 2 and tramples 8 onto the hero (5 - 8 = -3); the strike back deals 3 and
/// its Lifesteal heals p2's hero in the same simultaneous step (R176). `life_mage` puts a Radiant Life
/// Mage on p2's side, so that heal is 6 (R1180003) and the hero ends the combat at 3; without it the heal
/// is 3 and the hero ends at 0. `pawn` sets My Pawn in p2's backrow.
fn board(life_mage: bool, pawn: bool) -> (GameState, String, String) {
    jackioh_cards::register_all();
    let mut p2_field = vec![json!({ "def": VANILLA, "lane": 3, "statsOverride": { "attack": 3, "health": 2 } })];
    if life_mage {
        p2_field.push(json!({ "def": LIFE_MAGE, "lane": 1, "radiant": true }));
    }
    let mut p2 = json!({ "field": p2_field, "health": 5, "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA] });
    if pawn {
        p2["backrow"] = json!([MY_PAWN]);
    }
    let s = scenario(json!({
        "seed": "audit-lifesteal-heal", "active": "p1", "turn": 9,
        "p1": {
            "field": [{ "def": VANILLA, "lane": 3, "statsOverride": { "attack": 10, "health": 10 } }],
            "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA],
        },
        "p2": p2,
    }));
    let attacker = s.unit_at(P1, 3).id;
    let defender = s.unit_at(P2, 3).id;
    let mut state = s.state().clone();
    find_instance_mut(&mut state, &attacker).expect("attacker").granted_keywords.push(Keyword::Trample);
    find_instance_mut(&mut state, &defender).expect("defender").granted_keywords.push(Keyword::Lifesteal);
    (state, attacker, defender)
}

fn attack(state: &GameState, attacker: &str, target: &str) -> ReduceResult {
    let action = Action::new(
        ActionBody::Attack { attacker_id: attacker.to_string(), target_id: target.to_string() },
        P1,
        "audit-attack",
    );
    let after = reduce(state, &action);
    assert!(after.error.is_none(), "{:?}", after.error);
    after
}

fn overflow_kills(state: &GameState, attacker: &str, target: &str) -> bool {
    let entry = combat_forecast(state, P1)
        .attacks
        .into_iter()
        .find(|entry| entry.attacker_id == attacker && entry.target_id == target)
        .expect("the attack is forecast");
    entry.overflow.expect("the attack tramples onto the hero").kills
}

/// Control (passes): without the healing multiplier the projection and the combat agree, the hero falls.
#[test]
fn control_without_a_healing_multiplier_the_forecast_and_the_combat_agree() {
    let (state, attacker, defender) = board(false, false);
    assert!(overflow_kills(&state, &attacker, &defender));
    let after = attack(&state, &attacker, &defender);
    assert!(after.state.result.is_some(), "5 - 8 + 3 = 0: the hero falls");
}

/// Finding 1a. R176: "the hero it projects is the one the combat leaves: a defender's Lifesteal strike
/// back heals its controller in the same combat"; R1180003: the healing multiplier "reaches every heal of
/// an amount, a Spell's, a Unit's text and Lifesteal's alike"; R3100000: "no forecast can disagree with
/// the hit". `lethal::strike_back_heal` nets the strike back's raw damage, never the multiplied heal, so
/// the forecast says the Trample overflow ends the game while the combat leaves the hero at 3.
#[test]
fn r176_the_forecast_nets_a_lifesteal_strike_back_s_doubled_heal() {
    let (state, attacker, defender) = board(true, false);
    let after = attack(&state, &attacker, &defender);
    // The engine's combat: 5 - 8 (Trample excess) + 2 x 3 (Lifesteal, doubled) = 3, and the game goes on.
    assert!(after.state.result.is_none());
    assert_eq!(after.state.players[P2].hero.health, 3);
    assert!(
        !overflow_kills(&state, &attacker, &defender),
        "the forecast says the attack ends the game, and it leaves the hero at 3"
    );
}

/// Finding 1b. #96 My Pawn: "When the opponent declares an attack that would be lethal to your hero:
/// cancel it, ..."; R44 and R176 make "lethal" the hero the combat leaves. The same projection arms the
/// trap on this attack, which would have left its controller's hero at 3: it cancels a non-lethal attack
/// and hands the opponent's turn to the AI.
#[test]
fn r44_my_pawn_stays_set_when_the_doubled_lifesteal_heal_keeps_its_hero_alive() {
    let (state, attacker, defender) = board(true, true);
    let after = attack(&state, &attacker, &defender);
    let fired = after.events.iter().any(|event| matches!(event, GameEvent::TrapFired { .. }));
    assert!(!fired, "My Pawn fired on an attack that is not lethal to its hero");
}

const BUNKER: &str = "meditative-097-5"; // 5/10 Armor 3, First Strike, "This strikes Units in its lane with 3× its Attack."

/// Finding 2. §4.3 Lane strike (R1283, R94): Bunker's attack is multiplied by 3 "in combat with the
/// opposing Unit directly across in the same lane", its strike back included (`combat::resolve_combat`
/// multiplies it by `strike_back_multiplier_for`). R176: the lethal projection nets the heal of "a
/// defender's Lifesteal strike back". `lethal::strike_back_heal` projects that strike back at the
/// defender's plain attack, with no lane multiplier, so a Lifesteal Bunker's heal is read as 5 where the
/// combat heals 15: the forecast says the Trample overflow ends the game, and the hero is left at 10.
#[test]
fn r176_the_forecast_nets_a_lane_strike_defender_s_whole_lifesteal_heal() {
    jackioh_cards::register_all();
    let s = scenario(json!({
        "seed": "audit-lane-strike-heal", "active": "p1", "turn": 9,
        "p1": {
            "field": [{ "def": VANILLA, "lane": 3, "statsOverride": { "attack": 25, "health": 30 } }],
            "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA],
        },
        "p2": {
            "field": [{ "def": BUNKER, "lane": 3 }],
            "health": 7, "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA],
        },
    }));
    let attacker = s.unit_at(P1, 3).id;
    let defender = s.unit_at(P2, 3).id;
    let mut state = s.state().clone();
    find_instance_mut(&mut state, &attacker).expect("attacker").granted_keywords.push(Keyword::Trample);
    find_instance_mut(&mut state, &defender).expect("defender").granted_keywords.push(Keyword::Lifesteal);

    let after = attack(&state, &attacker, &defender);
    // The engine's combat: Bunker strikes first for 3 x 5 = 15 and its Lifesteal heals 15 (7 -> 22); the
    // blow of 25 less Armor 3 deals Bunker its 10 health and tramples 12 onto the hero (22 -> 10).
    assert!(after.state.result.is_none());
    assert_eq!(after.state.players[P2].hero.health, 10);
    assert!(
        !overflow_kills(&state, &attacker, &defender),
        "the forecast says the attack ends the game, and it leaves the hero at 10"
    );
}

const BLOOD_MOON: &str = "classicplus-022"; // Radiant, a Field Trap: "From now on, healing on enemies deals that much Pierce damage to them instead."

/// p1's 10/10 Trample attacker faces p2's 3/2 Lifesteal defender; p2's hero has 10 life. p1 holds a
/// face-up (public) Radiant Blood Moon, so the heal of the defender's Lifesteal strike back is dealt to
/// p2's hero as 3 Pierce damage instead (R413): 10 - 8 - 3 = -1, the hero falls.
fn blood_moon_board(pawn: bool) -> (GameState, String, String) {
    jackioh_cards::register_all();
    let mut p2 = json!({
        "field": [{ "def": VANILLA, "lane": 3, "statsOverride": { "attack": 3, "health": 2 } }],
        "health": 10, "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA],
    });
    if pawn {
        p2["backrow"] = json!([MY_PAWN]);
    }
    let s = scenario(json!({
        "seed": "audit-heal-converted", "active": "p1", "turn": 9,
        "p1": {
            "field": [{ "def": VANILLA, "lane": 3, "statsOverride": { "attack": 10, "health": 10 } }],
            "backrow": [{ "def": BLOOD_MOON, "radiant": true, "faceUp": true }],
            "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA],
        },
        "p2": p2,
    }));
    let attacker = s.unit_at(P1, 3).id;
    let defender = s.unit_at(P2, 3).id;
    let mut state = s.state().clone();
    find_instance_mut(&mut state, &attacker).expect("attacker").granted_keywords.push(Keyword::Trample);
    find_instance_mut(&mut state, &defender).expect("defender").granted_keywords.push(Keyword::Lifesteal);
    (state, attacker, defender)
}

/// Finding 3a. R176: the projection's hero "is the one the combat leaves"; R413: a face-up converter
/// turns the heal into Pierce damage. `lethal::strike_back_heal` still subtracts the strike back's heal,
/// so the forecast says the attack does not end the game, and it does.
#[test]
fn r176_the_forecast_counts_a_converted_lifesteal_heal_as_damage() {
    let (state, attacker, defender) = blood_moon_board(false);
    let after = attack(&state, &attacker, &defender);
    assert!(after.state.result.is_some(), "10 - 8 - 3 = -1: the hero falls");
    assert!(
        overflow_kills(&state, &attacker, &defender),
        "the forecast says the attack does not end the game, and it does"
    );
}

/// Finding 3b. #96 My Pawn: "When the opponent declares an attack that would be lethal to your hero:
/// cancel it". The attack above is lethal to p2's hero with nothing hidden on p1's side, and My Pawn
/// stays set: its controller loses with the trap unfired.
#[test]
fn r44_my_pawn_fires_on_an_attack_a_converted_lifesteal_heal_makes_lethal() {
    let (state, attacker, defender) = blood_moon_board(true);
    let after = attack(&state, &attacker, &defender);
    let fired = after.events.iter().any(|event| matches!(event, GameEvent::TrapFired { .. }));
    assert!(fired, "My Pawn stayed set on an attack that killed its hero");
}

const BANISHER: &str = "meditative-072"; // 2/1, "Whenever this damages a Unit, exile that Unit."

/// p1's 10/10 Trample attacker faces p2's The Banisher (2/1) with First Strike; p2's hero has 5 life.
/// The Banisher strikes first (§4.3 step 1) and its damage exiles the attacker (R1124), which "deals
/// nothing" (§4.3: "Whichever is destroyed here deals nothing"; `combat::has_fallen` counts a Unit marked
/// for exile), so no Trample excess reaches the hero. `pawn` sets My Pawn in p2's backrow.
fn banisher_board(pawn: bool) -> (GameState, String, String) {
    jackioh_cards::register_all();
    let mut p2 = json!({
        "field": [{ "def": BANISHER, "lane": 3 }],
        "health": 5, "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA],
    });
    if pawn {
        p2["backrow"] = json!([MY_PAWN]);
    }
    let s = scenario(json!({
        "seed": "audit-first-strike-exile", "active": "p1", "turn": 9,
        "p1": {
            "field": [{ "def": VANILLA, "lane": 3, "statsOverride": { "attack": 10, "health": 10 } }],
            "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA],
        },
        "p2": p2,
    }));
    let attacker = s.unit_at(P1, 3).id;
    let defender = s.unit_at(P2, 3).id;
    let mut state = s.state().clone();
    find_instance_mut(&mut state, &attacker).expect("attacker").granted_keywords.push(Keyword::Trample);
    find_instance_mut(&mut state, &defender).expect("defender").granted_keywords.push(Keyword::FirstStrike);
    (state, attacker, defender)
}

/// Finding 4. R176: "An attacker that a First Strike defender destroys in §4.3's step 1 deals nothing
/// at all, so none of its Trample excess can reach the hero and the attack is not lethal"; R4111000: a
/// First Strike defender's blow counts "an exile on damage as [it counts] Poisonous". The forecast's own
/// entry knows it (`forecast_attack`'s `exiled_first`), but R44's projection (`projected_damage_with`)
/// asks `falls_to_first_strike` alone, which does not: it counts 9 Trample excess that never lands, and
/// My Pawn cancels an attack that would have dealt its hero nothing.
#[test]
fn r176_my_pawn_stays_set_when_a_first_strike_banisher_exiles_the_attacker_first() {
    let (state, attacker, defender) = banisher_board(false);
    let after = attack(&state, &attacker, &defender);
    // The engine's combat: the attacker is exiled before its blow, and the hero is untouched.
    assert_eq!(after.state.players[P2].hero.health, 5);
    assert!(find_instance(&after.state, &defender).is_some_and(|card| matches!(card.zone, Zone::Field { .. })));

    let (state, attacker, defender) = banisher_board(true);
    let after = attack(&state, &attacker, &defender);
    let fired = after.events.iter().any(|event| matches!(event, GameEvent::TrapFired { .. }));
    assert!(!fired, "My Pawn fired on an attack that deals its hero nothing");
}
