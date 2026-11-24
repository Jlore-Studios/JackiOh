//! Audit of the engine's continuous effects and the effects that change a card: each test asserts the
//! ruled behaviour and fails on the code as it stands.
//!
//! 1. A Transform's replacement (`effects/transform.rs::replace_on_field`, `zones::replace_in_zone`)
//!    skips the steps every other arrival on the field runs: Dormant (R1839000), a Starship hull's
//!    memory, and the arrival roll of a card with no Cry (R3134002).
//! 2. A card made Radiant where it stands (`effects/radiant.rs::make_radiant`) gains its Radiant face's
//!    Divine Shield (`gain_printed_shield`) but neither starts a Brittle that face newly prints nor
//!    gives back a used Reborn for one it newly prints (§5.2: "newly gained keywords apply at once").

use jackioh_engine::testkit::*;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;
const FILLER: &str = "core-005";
/// Stinkyscape+ #103 Transcend: "Transform your permanents into ones that cost (1) more."
const TRANSCEND: &str = "stinkyscapeplus-103";
/// Stinkyscape #19 Imprisoned Cosmality, (2) 10/6 Pierce: "Dormant for 2 turns. When this awakens, …".
const COSMALITY: &str = "stinkyscape-019";
/// Stinkyscape+ #62 Overdrive Projector, a (3) Field Spell: "Dormant for N turns. Your Units gain Rush
/// and Windfury, and your Spells gain Echo +N."
const PROJECTOR: &str = "stinkyscapeplus-062";
/// Stinkyscape+ #157 Phantom Shipyard, a (4) 0/0 Unit: "Dormant until launched."
const SHIPYARD: &str = "stinkyscapeplus-157";
/// Core #98 Heroic Power, a (1) Field Spell: "Indestructible. Cry: Discover one of 14 powers."
const HEROIC_POWER: &str = "core-098";
/// Classic+ #65.4 Golden Grape: "Make a card on your side of the field or in your hand Radiant."
const GRAPE: &str = "classicplus-065-4";

fn transcend_board(seed: &str, field: Value, backrow: Value) -> Scenario {
    scenario(json!({
        "seed": seed,
        "p1": { "hand": [TRANSCEND, FILLER], "field": field, "backrow": backrow,
                "library": [FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER, FILLER] },
    }))
}

/// The first of 900 seeded boards on which Transcend turned P1's one permanent into `into`: the
/// scenario after the cast, and the new card's id as the `transformed` event named it.
fn transcended_into(name: &str, field: Value, backrow: Value, into: &str) -> (Scenario, String) {
    for n in 0..900 {
        let mut s = transcend_board(&format!("{name}-{n}"), field.clone(), backrow.clone());
        s.play(TRANSCEND, json!({}));
        let made = s.events_of("transformed").into_iter().find_map(|event| {
            let event = serde_json::to_value(event).expect("an event is JSON");
            (event["toDefId"] == into).then(|| event["newInstanceId"].as_str().unwrap_or_default().to_string())
        });
        if let Some(id) = made {
            return (s, id);
        }
    }
    panic!("no seed of 900 drew {into}");
}

/// A shipped Unit of this printed price.
fn unit_costing(price: i32) -> String {
    jackioh_engine::catalog::query(&json_as(json!({ "type": CardType::Unit, "cost": price })))
        .first()
        .map(|def| def.id.clone())
        .expect("a Unit of that price")
}

/// R1839000 (§6.1 Dormant): "A Unit that prints it … enters the field Dormant as it arrives from off the
/// field, any way it arrives: played, summoned, put there by a setup." A Transform's result is a new
/// card that has entered the field (§6.3 Replace; `replace_in_zone` starts its printed Brittle for that
/// reason), so one that prints Dormant is Dormant. The control plays the same card from a hand.
#[test]
fn r1839000_a_unit_transformed_into_one_that_prints_dormant_is_dormant() {
    jackioh_cards::register_all();
    let mut control = scenario(json!({
        "seed": "control",
        "p1": { "hand": [COSMALITY, FILLER], "library": [FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER, FILLER] },
    }));
    control.play(COSMALITY, json!({}));
    assert!(control.unit_at(P1, 1).slumber.is_some(), "control: played from a hand it is Dormant");

    // core-003 Right-house defender costs (1); Transcend makes it a random (2)-cost Unit.
    let (s, id) = transcended_into("dormant-unit", json!(["core-003"]), json!([]), COSMALITY);
    assert!(
        s.card(id.as_str()).slumber.is_some(),
        "R1839000: Imprisoned Cosmality arrived by a Transform and is not Dormant"
    );
}

/// R3556000, R1839000: the same for a face-up Field Spell that prints Dormant. Overdrive Projector's
/// "Your Units gain Rush and Windfury" is in force at once instead of after its countdown.
#[test]
fn r1839000_a_field_spell_transformed_into_one_that_prints_dormant_is_dormant() {
    jackioh_cards::register_all();
    // classic-004 Palantir is a (2) Field Spell; Transcend makes it a random (3)-cost Field Spell.
    let (s, id) = transcended_into("dormant-fs", json!([]), json!(["classic-004"]), PROJECTOR);
    assert!(
        s.card(id.as_str()).slumber.is_some(),
        "R1839000: Overdrive Projector arrived by a Transform and is not Dormant"
    );
}

/// R1839000: Phantom Shipyard is a 0/0 that is "Dormant until launched", so no state check reads it at
/// 0 health. Made by a Transform it is awake at 0/0 and the state check kills it at once.
#[test]
fn r1839000_a_unit_transformed_into_phantom_shipyard_is_dormant_and_stays() {
    jackioh_cards::register_all();
    let mut control = scenario(json!({
        "seed": "control",
        "p1": { "hand": [SHIPYARD, FILLER], "library": [FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER, FILLER] },
    }));
    control.play(SHIPYARD, json!({}));
    assert!(
        control.unit(P1, 1).is_some_and(|card| card.def_id == SHIPYARD && card.slumber.is_some()),
        "control: played from a hand it lies Dormant on the field"
    );

    let (s, id) = transcended_into("shipyard", json!([unit_costing(3)]), json!([]), SHIPYARD);
    let new = s.card(id.as_str());
    assert_eq!(
        new.zone.z(),
        ZoneName::Field,
        "R1839000: the Phantom Shipyard a Transform made is not Dormant and died at 0/0"
    );
    assert!(new.slumber.is_some(), "R1839000: it is Dormant");
}

/// R3134002: "A copy that reaches the field with no Cry rolls one. A copy Summoned, Recruited, fused
/// onto a card on the field, Reborn or Flickered back has no Cry to choose with, so R151's arrival roll
/// stays for it … so a copy is never dead." A Transform fires no Cry (R1), so a Heroic Power a Transform
/// makes on the field must hold a power (`memory.power`); without one it lists no Activate, for good.
#[test]
fn r3134002_a_heroic_power_made_by_a_transform_rolls_its_power() {
    jackioh_cards::register_all();
    // stinkyscapeplus-040 Mender is a (0) Field Spell; Transcend makes it a random (1)-cost one.
    let (s, id) = transcended_into("power", json!([]), json!(["stinkyscapeplus-040"]), HEROIC_POWER);
    let new = s.card(id.as_str());
    assert!(s.state().pending.is_none(), "no prompt is open: nothing is still to choose");
    assert!(
        new.memory.contains_key("power"),
        "R3134002: a Heroic Power that reached the field by a Transform holds no power"
    );
}

/// §5.2: a card made Radiant on the field has its "newly gained keywords apply at once". Stinkyscape
/// #135.1 Sack prints Brittle 1 on its Radiant face only, and B3.3 (R385) has a Brittle count tick on
/// the field and crumble its card at 0. A Sack that entered Radiant crumbles after its turn cycle (the
/// control); one made Radiant where it stands shows Brittle 1 to both seats and must crumble too.
#[test]
fn s5_2_a_radiant_face_that_newly_prints_brittle_starts_its_count() {
    jackioh_cards::register_all();
    const SACK: &str = "stinkyscape-135-1"; // 0/4 Can't Defend; Radiant 0/8 Can't Defend, Brittle 1
    let board = |radiant: bool| {
        scenario(json!({
            "seed": "sack",
            "p1": { "hand": [GRAPE, FILLER], "field": [{ "def": SACK, "radiant": radiant }],
                    "library": [FILLER, FILLER, FILLER, FILLER, FILLER], "mana": 10 },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER, FILLER, FILLER] },
        }))
    };
    let cycle = |s: &mut Scenario| {
        s.end_turn(); // P1's turn ends, P2's starts
        s.end_turn(); // P2's turn ends, P1's starts: the first tick of a count started two turns before
    };

    let mut control = board(true);
    assert!(control.card(SACK).brittle.is_some(), "control: its printed Brittle started");
    cycle(&mut control);
    assert!(control.unit(P1, 1).is_none(), "control: a Sack that entered Radiant crumbles");

    let mut s = board(false);
    let id = s.card(SACK).id.clone();
    let targets = pick::card(&s, id.as_str());
    s.play(GRAPE, json!({ "targets": targets }));
    assert!(s.card(id.as_str()).radiant, "Golden Grape made it Radiant");
    assert!(
        s.stats(id.as_str()).keywords.contains(&Keyword::Brittle { n: 1 }),
        "it shows its Radiant face's Brittle 1"
    );
    cycle(&mut s);
    assert!(
        s.unit(P1, 1).is_none(),
        "§5.2, R385: the Sack shows Brittle 1 since it was made Radiant and has not crumbled a turn cycle later \
         (its count: {:?})",
        s.unit(P1, 1).and_then(|card| card.brittle)
    );
}

/// §5.2: "newly gained keywords apply at once". Classic+ #6 Wrong-House Attacker prints Reborn on its
/// Radiant face only. A base one that came back through a Reborn an effect had given it (the grant went
/// with R78's reset; `rebornSpent` stayed on the body, §4.5 step 4) and is then made Radiant has its
/// Radiant face's Reborn: §10.4 drops a used Reborn only "until granted again", and
/// `radiant::gain_printed_shield` gives a spent Divine Shield back for a face that newly prints one.
#[test]
fn s5_2_a_radiant_face_that_newly_prints_reborn_gives_it_to_a_body_that_used_one() {
    jackioh_cards::register_all();
    const ATTACKER: &str = "classicplus-006"; // 1/1 Rush, Lifesteal, Poisonous; Radiant adds Reborn
    let mut s = scenario(json!({
        "seed": "reborn-face",
        "p1": { "hand": [GRAPE, FILLER], "field": [ATTACKER], "library": [FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "field": [{ "def": "core-008", "lane": 1 }],
                "library": [FILLER, FILLER, FILLER, FILLER] },
    }));
    // An effect's grant, written as `buff::grant_onto` writes it (Core #63 Plastic Surgery's and #80
    // Zao Gao's random keywords are drawn from R21's pool, which holds Reborn).
    let id = s.card(ATTACKER).id.clone();
    s.card_mut(id.as_str()).granted_keywords.push(Keyword::Reborn);
    s.card_mut(id.as_str()).summoned_turn = None;
    let enemy = s.unit_at(P2, 1).id;
    s.attack(id.as_str(), enemy.as_str());
    let back = s.unit(P1, 1).expect("the Reborn body is back in its lane");
    assert_eq!(back.reborn_spent, Some(true), "it came back through Reborn");
    assert!(!back.radiant && back.granted_keywords.is_empty(), "base, the grant gone (R78)");

    let targets = pick::card(&s, back.id.as_str());
    s.play(GRAPE, json!({ "targets": targets }));
    let now = s.unit_at(P1, 1);
    assert!(now.radiant, "Golden Grape made it Radiant");
    assert!(
        s.stats(now.id.as_str()).keywords.contains(&Keyword::Reborn),
        "§5.2: the Radiant face newly prints Reborn, so the card has it; it has {:?}",
        s.stats(now.id.as_str()).keywords
    );
}
