//! Audit of the engine's continuous effects and the effects that change a card. Exploratory, temporary.

use jackioh_engine::testkit::*;

const P1: PlayerId = PlayerId::P1;
const FILLER: &str = "core-005";
/// Stinkyscape+ #103 Transcend: "Transform your permanents into ones that cost (1) more."
const TRANSCEND: &str = "stinkyscapeplus-103";
/// Stinkyscape #19 Imprisoned Cosmality, (2) 10/6 Pierce: "Dormant for 2 turns. When this awakens, …".
const COSMALITY: &str = "stinkyscape-019";
/// Stinkyscape+ #62 Overdrive Projector, a (3) Field Spell: "Dormant for N turns. Your Units gain Rush
/// and Windfury, and your Spells gain Echo +N."
const PROJECTOR: &str = "stinkyscapeplus-062";

fn transcend_board(seed: &str, field: Value, backrow: Value) -> Scenario {
    scenario(json!({
        "seed": seed,
        "p1": { "hand": [TRANSCEND, FILLER], "field": field, "backrow": backrow,
                "library": [FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER, FILLER] },
    }))
}

/// Control: the same card played from a hand enters Dormant (R1839000).
#[test]
fn control_a_played_imprisoned_cosmality_enters_dormant() {
    jackioh_cards::register_all();
    let mut s = scenario(json!({
        "seed": "control",
        "p1": { "hand": [COSMALITY, FILLER], "library": [FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER, FILLER] },
    }));
    s.play(COSMALITY, json!({}));
    let unit = s.unit(P1, 1);
    println!("played: {:?}", unit.as_ref().map(|u| (u.def_id.clone(), u.slumber)));
}

/// R1839000: "A Unit that prints it enters the field Dormant as it arrives from off the field, any way
/// it arrives". A Transform's result is a new card that has entered the field (§6.3 Replace; the same
/// function starts its printed Brittle for that reason), so one that prints Dormant is Dormant.
#[test]
fn r1839000_a_unit_transformed_into_one_that_prints_dormant_is_dormant() {
    jackioh_cards::register_all();
    let mut found = false;
    for n in 0..600 {
        // core-003 Right-house defender costs (1); Transcend makes it a random (2)-cost Unit.
        let mut s = transcend_board(&format!("dormant-unit-{n}"), json!(["core-003"]), json!([]));
        s.play(TRANSCEND, json!({}));
        let Some(new) = s.unit(P1, 1) else { continue };
        if new.def_id != COSMALITY {
            continue;
        }
        found = true;
        println!("seed {n}: {} slumber {:?}", new.def_id, new.slumber);
        assert!(
            new.slumber.is_some(),
            "R1839000: Imprisoned Cosmality arrived by a Transform and is not Dormant (seed {n})"
        );
        break;
    }
    assert!(found, "no seed drew Imprisoned Cosmality");
}

#[test]
fn r1839000_a_field_spell_transformed_into_one_that_prints_dormant_is_dormant() {
    jackioh_cards::register_all();
    let mut found = false;
    for n in 0..600 {
        // classic-004 Palantir is a (2) Field Spell; Transcend makes it a random (3)-cost Field Spell.
        let mut s = transcend_board(&format!("dormant-fs-{n}"), json!([]), json!(["classic-004"]));
        s.play(TRANSCEND, json!({}));
        let Some(new) = s.backrow(P1, 1) else { continue };
        if n < 3 {
            println!("seed {n}: backrow now {}", new.def_id);
        }
        if new.def_id != PROJECTOR {
            continue;
        }
        found = true;
        println!("seed {n}: {} slumber {:?}", new.def_id, new.slumber);
        assert!(
            new.slumber.is_some(),
            "R1839000: Overdrive Projector arrived by a Transform and is not Dormant (seed {n})"
        );
        break;
    }
    assert!(found, "no seed drew Overdrive Projector");
}

/// R3134002: "A copy that reaches the field with no Cry rolls one … so a copy is never dead." A
/// Transform fires no Cry (R1), so a Heroic Power a Transform makes on the field must hold a power.
#[test]
fn r3134002_a_heroic_power_made_by_a_transform_rolls_its_power() {
    jackioh_cards::register_all();
    let mut found = false;
    for n in 0..600 {
        // stinkyscapeplus-040 Mender is a (0) Field Spell; Transcend makes it a random (1)-cost one.
        let mut s = transcend_board(&format!("power-{n}"), json!([]), json!(["stinkyscapeplus-040"]));
        s.play(TRANSCEND, json!({}));
        let Some(new) = s.backrow(P1, 1) else { continue };
        if new.def_id != "core-098" {
            continue;
        }
        found = true;
        println!("seed {n}: {} memory {:?} pending {:?}", new.def_id, new.memory, s.state().pending.is_some());
        assert!(
            new.memory.contains_key("power"),
            "R3134002: a Heroic Power that reached the field by a Transform holds no power (seed {n})"
        );
        break;
    }
    assert!(found, "no seed drew Heroic Power");
}

#[test]
fn probe_more_transform_arrivals() {
    jackioh_cards::register_all();
    // Control: Rod of Ages played from a hand.
    let mut c = scenario(json!({
        "seed": "rod",
        "p1": { "hand": ["stinkyscape-016", FILLER], "library": [FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER, FILLER] },
    }));
    c.play("stinkyscape-016", json!({}));
    let rod = c.card("stinkyscape-016").clone();
    println!("played Rod of Ages: zone {:?} pos {:?}", rod.zone, rod.position);
    let mut seen_rod = false;
    for n in 0..600 {
        let mut s = transcend_board(&format!("rod-{n}"), json!([]), json!(["stinkyscapeplus-040"]));
        s.play(TRANSCEND, json!({}));
        let all: Vec<CardInstance> = (1..=5).filter_map(|l| s.backrow(P1, l)).chain((1..=5).filter_map(|l| s.unit(P1, l))).collect();
        for new in all {
            if (new.def_id == "stinkyscape-016" || new.def_id == "stinkyscapeplus-087") && !seen_rod {
                seen_rod = true;
                println!("seed {n}: transformed into {} zone {:?} pos {:?}", new.def_id, new.zone, new.position);
            }
        }
        if seen_rod { break; }
    }
    // Phantom Shipyard: a (4) 0/0 Unit, "Dormant until launched".
    let three = jackioh_engine::catalog::query(&json_as(json!({ "type": CardType::Unit, "cost": 3 })))
        .first().map(|def| def.id.clone()).expect("a (3) Unit");
    for n in 0..900 {
        let mut s = transcend_board(&format!("yard-{n}"), json!([three.clone()]), json!([]));
        s.play(TRANSCEND, json!({}));
        let made: Vec<GameEvent> = s.events_of("transformed");
        let hit = made.iter().any(|e| serde_json::to_value(e).unwrap()["toDefId"] == "stinkyscapeplus-157");
        if hit {
            let unit = s.unit(P1, 1);
            println!("seed {n}: Phantom Shipyard after Transcend: on field {:?}; graveyard {:?}",
                unit.map(|u| (u.def_id, u.slumber, u.starship.is_some())),
                s.pile(P1, "graveyard").iter().map(|c| c.def_id.clone()).collect::<Vec<_>>());
            break;
        }
    }
}

/// §5.2: a card made Radiant on the field has its "newly gained keywords apply at once". Classic+ #6
/// Wrong-House Attacker prints Reborn on its Radiant face only, so a base one that came back through
/// a Reborn an effect had given it (the grant went with R78's reset; `rebornSpent` stayed on the body)
/// and is then made Radiant has its Radiant face's Reborn, as `radiant::gain_printed_shield` gives a
/// spent Divine Shield back for a face that newly prints one.
#[test]
fn s5_2_a_radiant_face_that_newly_prints_reborn_gives_it_to_a_body_that_used_one() {
    jackioh_cards::register_all();
    const ATTACKER: &str = "classicplus-006"; // 1/1 Rush, Lifesteal, Poisonous; Radiant adds Reborn
    const GRAPE: &str = "classicplus-065-4"; // "Make a card on your side of the field or in your hand Radiant."
    let mut s = scenario(json!({
        "seed": "reborn-face",
        "p1": { "hand": [GRAPE, FILLER], "field": [ATTACKER], "library": [FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "field": [{ "def": "core-008", "lane": 1 }], "library": [FILLER, FILLER, FILLER, FILLER] },
    }));
    // An effect's grant (Core #63 Plastic Surgery's random keyword, R21's pool holds Reborn).
    let id = s.card(ATTACKER).id.clone();
    s.card_mut(id.as_str()).granted_keywords.push(Keyword::Reborn);
    s.card_mut(id.as_str()).summoned_turn = None;
    let enemy = s.unit_at(PlayerId::P2, 1).id;
    s.attack(id.as_str(), enemy.as_str());
    let back = s.unit(P1, 1).expect("the Reborn body is back in its lane");
    println!("back: {} radiant {} spent {:?} kw {:?}", back.def_id, back.radiant, back.reborn_spent, s.stats(back.id.as_str()).keywords);
    assert_eq!(back.reborn_spent, Some(true), "it came back through Reborn");
    assert!(!back.radiant && back.granted_keywords.is_empty(), "base, the grant gone (R78)");
    let targets = pick::card(&s, back.id.as_str());
    s.play(GRAPE, json!({ "targets": targets }));
    let now = s.unit_at(P1, 1);
    println!("radiant: {} kw {:?}", now.radiant, s.stats(now.id.as_str()).keywords);
    assert!(now.radiant, "Golden Grape made it Radiant");
    assert!(
        s.stats(now.id.as_str()).keywords.contains(&Keyword::Reborn),
        "§5.2: the Radiant face newly prints Reborn, so the card has it"
    );
}
