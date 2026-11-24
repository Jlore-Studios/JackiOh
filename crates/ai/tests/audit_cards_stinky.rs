//! Audit (Stinkyscape cards, S #1 to #177 and the set's tokens): each test asserts the printed or
//! ruled behaviour and fails on the code as it stands.

use jackioh_engine::testkit::*;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

const FILLER: &str = "core-005"; // Stockpile: a Spell, so no turn ends by itself
const TNACK: &str = "stinkyscape-020"; // Tnack Trav: "Cry: Copy the text of a Legendary or Mythic Unit on the field."

fn scenario_with_cards(opts: serde_json::Value) -> Scenario {
    jackioh_cards::register_all();
    let mut s = scenario(opts);
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

fn filler(n: usize) -> Vec<&'static str> {
    vec![FILLER; n]
}

/// The keywords the layers give `card` now, but its printed Equip (Keenus's, which a copy carries).
fn combat_keywords(s: &Scenario, card: &CardInstance) -> Vec<Keyword> {
    s.stats(card).keywords.into_iter().filter(|keyword| *keyword != Keyword::Equip).collect()
}

/// S #20 Tnack Trav copies "the text of a Legendary or Mythic Unit": R1850000 gives it "its printed
/// keywords, its aura, its triggers …". M #65 Keymaster Keenus (Legendary) prints "Aura: Has 5 random
/// combat keywords", and R3778000 holds that Aura to its number: "Its keywords now always match the
/// number its text shows", a kept card drawing its own as a Fuse hands it the text. The copy draws
/// nothing: Tnack Trav reads "Has 5 random combat keywords" and has none, for its whole stay.
#[test]
fn tnack_trav_copying_keenus_has_the_auras_five_combat_keywords() {
    const KEENUS: &str = "meditative-065";
    let mut s = scenario_with_cards(json!({
        "p1": { "hand": [KEENUS, TNACK, FILLER], "library": filler(6), "mana": 10 },
        "p2": { "hand": [FILLER, FILLER], "library": filler(6) },
    }));
    s.play(KEENUS, json!({ "zone": 1 }));
    let keenus = s.unit_at(P1, 1);
    assert_eq!(combat_keywords(&s, &keenus).len(), 5, "Keenus, played, drew its Aura's five");
    s.state_mut().players[P1].mana.current = 10;
    s.play(TNACK, json!({ "zone": 3, "targets": pick::instance(&keenus) }));
    let tnack = s.unit_at(P1, 3);
    assert_eq!(tnack.def_id, TNACK);
    assert_eq!(
        combat_keywords(&s, &tnack).len(),
        5,
        "R1850000, R3778000: the copied Aura \"Has 5 random combat keywords\" gives Tnack Trav five; it has {:?}",
        s.stats(&tnack).keywords
    );
}

/// The same for the set's own S #111 Shockwave447 (Mythic): "Aura: Has 3 random combat keywords."
#[test]
fn tnack_trav_copying_shockwave447_has_the_auras_three_combat_keywords() {
    const SHOCKWAVE: &str = "stinkyscape-111";
    let mut s = scenario_with_cards(json!({
        "p1": { "hand": [TNACK, FILLER], "library": filler(6), "mana": 10 },
        "p2": { "hand": [FILLER, FILLER], "field": [{ "def": SHOCKWAVE, "lane": 1 }], "library": filler(6) },
    }));
    let shockwave = s.unit_at(P2, 1);
    s.play(TNACK, json!({ "zone": 3, "targets": pick::instance(&shockwave) }));
    let tnack = s.unit_at(P1, 3);
    // A round later it is the same: nothing ever draws them.
    s.end_turn();
    s.end_turn();
    assert_eq!(
        combat_keywords(&s, &tnack).len(),
        3,
        "R1850000, R3778000: the copied Aura \"Has 3 random combat keywords\" gives Tnack Trav three; it has {:?}",
        s.stats(&tnack).keywords
    );
}

/// S #9 Ticketed: base "Brittle 3 / Stack this on a permanent anywhere.", Radiant "Stack this on a
/// permanent anywhere." R1875001: "The Radiant face has no Brittle, so it holds until it is removed."
/// A count a printed Brittle started is the card's text (`brittle_count::active_brittle_count`: a
/// Vanilla switches it off), so a Ticketed made Radiant in place (S #110 iKwzki's "make it Radiant
/// next turn") wears a face with no Brittle and must stop crumbling. Its base face's count ticks on
/// and destroys it.
#[test]
fn ticketed_made_radiant_in_place_no_longer_crumbles() {
    const TICKETED: &str = "stinkyscape-009";
    const IKWZKI: &str = "stinkyscape-110";
    const HANGER: &str = "stinkyscape-168"; // Drone Hanger: a face-up Field Spell to stack on
    let board = |ticketed_radiant: bool| {
        scenario_with_cards(json!({
            "p1": {
                "hand": [{ "def": TICKETED, "radiant": ticketed_radiant }, IKWZKI, FILLER, FILLER, FILLER],
                "backrow": [{ "def": HANGER, "faceUp": true, "lane": 2 }],
                "library": filler(14),
                "mana": 10,
            },
            "p2": { "hand": [FILLER, FILLER, FILLER], "library": filler(14) },
        }))
    };
    let on_field = |s: &Scenario, card: &CardInstance| matches!(s.card(card).zone, Zone::Field { .. });

    // The control: a Ticketed played Radiant stands through four of its controller's turns.
    let mut s = board(true);
    let hanger = s.backrow(P1, 2).expect("the Drone Hanger");
    s.play(TICKETED, pick::targets(pick::instance(&hanger)));
    let ticketed = s.card(TICKETED).clone();
    for _ in 0..4 {
        s.end_turn();
        s.end_turn();
    }
    assert!(on_field(&s, &ticketed), "a Radiant Ticketed has no Brittle");

    // A base Ticketed, made Radiant in place at the start of p1's next turn by iKwzki's Duo.
    let mut s = board(false);
    let hanger = s.backrow(P1, 2).expect("the Drone Hanger");
    s.play(TICKETED, pick::targets(pick::instance(&hanger)));
    let ticketed = s.card(TICKETED).clone();
    s.play(IKWZKI, json!({ "zone": 1, "targets": pick::instance(&ticketed) }));
    s.end_turn();
    s.end_turn();
    assert!(s.card(&ticketed).radiant, "iKwzki's Duo made it Radiant at the start of p1's next turn");
    for _ in 0..3 {
        s.end_turn();
        s.end_turn();
    }
    assert!(
        on_field(&s, &ticketed),
        "R1875001: the Radiant face has no Brittle, so it holds until it is removed; it is in {:?} with {} `crumbled`",
        s.card(&ticketed).zone,
        s.count("crumbled")
    );
}

/// S #10.1 Inner Fire, "Set a Unit's Attack equal to its Health." R1867002: "Attack becomes Health plus
/// the running face's shift, with a floor of 0", read "through the §10.4 layers", auras included.
/// R1868001 (S #118 Secure System, "S #10.1 Inner Fire's reading") sets a number exactly by reading the
/// Attack before layer 5's floor, "one Nerfed below 0 included". Inner Fire reads the floored Attack, so
/// a Unit whose Attack stands below 0 under the 0 it shows (a 1/5 under Core #65.1 Spikey Pillow's
/// "Your Units have −2 Attack") ends short of its Health by what it was below 0.
fn inner_fire_under_a_pillow(radiant: bool) -> (i32, i32) {
    const INNER_FIRE: &str = "stinkyscape-010-1";
    const PILLOW: &str = "core-065-1";
    let mut s = scenario_with_cards(json!({
        "p1": {
            "hand": [{ "def": INNER_FIRE, "radiant": radiant }, FILLER],
            "field": [{ "def": "stand-in (1) 1/5", "lane": 1 }, { "def": PILLOW, "lane": 3 }],
            "library": filler(4),
            "mana": 10,
        },
        "p2": { "hand": [FILLER], "library": filler(4) },
    }));
    let unit = s.unit_at(P1, 1);
    assert_eq!(s.stats(&unit).attack, 0, "1 Attack under −2 shows 0");
    s.play(INNER_FIRE, pick::targets(pick::instance(&unit)));
    let after = s.stats(&unit);
    (after.attack, after.health)
}

#[test]
fn inner_fire_sets_attack_to_health_on_a_unit_below_zero_attack() {
    let (attack, health) = inner_fire_under_a_pillow(false);
    assert_eq!(attack, health, "R1867002: Attack becomes Health");
}

/// The Radiant face: "… plus {shift} if it's yours", shift 4.
#[test]
fn radiant_inner_fire_sets_attack_to_health_plus_four_on_a_unit_below_zero_attack() {
    let (attack, health) = inner_fire_under_a_pillow(true);
    assert_eq!(attack, health + 4, "R1867002: Attack becomes Health plus the shift");
}
