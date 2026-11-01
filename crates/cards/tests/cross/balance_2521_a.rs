//! Issue #2521, "Patch: Stinkyscape balance changes I": the spec-tester's suite for behaviours B1–B20 of
//! `.fullsend/SPEC.md`, written against SPEC's surface before the code that makes them true. Each test
//! names the behavior it holds the code to (`b<n>_…`); each card's own file still holds its own tests.
//!
//!  - B1–B5, the Resources: their tags and prices, Korrelite's Pierce and Echo, Narcor's Legendary pool,
//!    Red Narcor's mana and draw.
//!  - B6–B8, the Factions and Swear Loyalities, by SPEC S9's faction table: a faction is its Faction card
//!    plus the cards it adds; Swear exiles the other factions' cards from its caster's hand and library.
//!  - B9–B16, the Starship Pieces: the fifteen that are no pieces, the base version a dead piece fuses
//!    (SPEC S4), Stratos, S #177 Cloaking Module, Paragon-X, Jlockeed's Yamato Cannon.
//!  - B17–B20, the weapons: no hero target, their damage and their uses, Flak's weighted rolls (S1, S7).
//!
//! No turn ends under a test by itself: each side holds a play, and `game` turns R82's auto-end off.

use jackioh_cards::card_def;
use jackioh_engine::testkit::*;

use super::{scenario, targets_of};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

// Cards from other sets, there for their bodies alone.
const ANCHOR: &str = "core-010"; // Rapid Replenish, a (0) Spell: a play always left in hand.
const FILLER: &str = "core-005"; // Stockpile, a (1) Spell: library filler for the draws.
const VANILLA: &str = "core-008"; // Mr. Vanilla, a (1) 4/4 Unit with no text.
const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
const MENACE: &str = "core-019"; // Midrange Menace, a (3) 9/9 Taunt Unit.
const SEVEN: &str = "core-025"; // 4-mana 7/7, a (4) 7/7 Unit with Armor 7.
const BIG_FELINOR: &str = "core-043"; // (4) Unit 3/10, Radiant 6/20. Cry: Destroy all non-Felinor Units.
const TWISTING_NETHER: &str = "core-088"; // (4) Spell: Destroy all permanents.
const CAT_EARS: &str = "meditative-067"; // Sentient Cat Ears, a (1) 1/1 Equip Unit that is no Starship Piece.

// B1–B5: the Resources.
const REKNITE: &str = "stinkyscape-t-reknite";
const KORRELITE: &str = "stinkyscape-t-korrelite";
const GELLIUM: &str = "stinkyscape-t-gellium";
const AXNIT: &str = "stinkyscape-t-axnit";
const NARCOR: &str = "stinkyscape-t-narcor";
const RED_NARCOR: &str = "stinkyscape-t-red-narcor";
const RESOURCES: [&str; 6] = [REKNITE, KORRELITE, GELLIUM, AXNIT, NARCOR, RED_NARCOR];
/// B4: Narcor's plays per face.
const NARCOR_PLAYS: usize = 60;

// B6–B8: the Factions (SPEC S9).
const SWEAR: &str = "stinkyscape-163";
const ETHICS: &str = "stinkyscape-159";
const VANGUARD: &str = "stinkyscape-095";
const GOLDEN: &str = "stinkyscape-160";
const PEGASUS: &str = "stinkyscape-161";
const STAR_JUMPER: &str = "stinkyscape-162";
const TYLANNT: &str = "stinkyscape-006";

/// A Faction card, its name, and the cards it adds (SPEC S9).
type Faction = (&'static str, &'static str, &'static [&'static str]);

const FACTIONS: [Faction; 5] = [
    (
        ETHICS,
        "Faction: ETHICS",
        &[
            "stinkyscape-110",
            "stinkyscape-111",
            "stinkyscape-109",
            "stinkyscape-020",
            "stinkyscape-037",
            TYLANNT,
        ],
    ),
    (
        VANGUARD,
        "Faction: Vanguard Coalition",
        &[
            "stinkyscape-115",
            "stinkyscape-096",
            TYLANNT,
            "stinkyscape-103",
            "stinkyscape-087",
            "stinkyscape-088",
        ],
    ),
    (
        GOLDEN,
        "Faction: Golden Accord",
        &[
            "stinkyscape-072",
            "stinkyscape-073",
            "stinkyscape-074",
            "stinkyscape-069",
        ],
    ),
    (
        PEGASUS,
        "Faction: Pegasus",
        &[
            "stinkyscape-113",
            "stinkyscape-094",
            "stinkyscape-123",
            "stinkyscape-114",
        ],
    ),
    (
        STAR_JUMPER,
        "Faction: Star-Jumper Core",
        &[
            "stinkyscape-075",
            "stinkyscape-076",
            "stinkyscape-078",
            "stinkyscape-079",
            "stinkyscape-080",
        ],
    ),
];

// B9–B16: the Starship Pieces and their neighbors.
const CANNON_V3: &str = "stinkyscape-052";
const FLAK_V3: &str = "stinkyscape-053";
const STRATOS: &str = "stinkyscape-068";
const PARAGON_X: &str = "stinkyscape-074";
const YAMATO: &str = "stinkyscape-128";
const STRATOS_SWARM: &str = "stinkyscape-158";
const CLOAKING_MODULE: &str = "stinkyscape-177";
/// B9: the fifteen cards that are no Starship Pieces any more.
const NOT_PIECES: [&str; 15] = [
    "stinkyscape-069",
    "stinkyscape-072",
    "stinkyscape-073",
    "stinkyscape-074",
    "stinkyscape-075",
    "stinkyscape-076",
    "stinkyscape-077",
    "stinkyscape-078",
    "stinkyscape-080",
    "stinkyscape-081",
    "stinkyscape-082",
    "stinkyscape-083",
    "stinkyscape-085",
    "stinkyscape-086",
    "stinkyscape-068",
];

// B17–B20: the weapons.
/// B17, B18: the weapons that aim at one enemy Unit — id, name, damage on the base and Radiant faces.
const AIMED_WEAPONS: [(&str, &str, i32, i32); 9] = [
    ("stinkyscape-050", "Beam v3", 5, 10),
    ("stinkyscape-054", "Beam v4", 7, 14),
    // #2657 took Beam v5 to 11 and 22.
    ("stinkyscape-058", "Beam v5", 11, 22),
    (CANNON_V3, "Cannon v3", 5, 10),
    ("stinkyscape-056", "Cannon v4", 7, 14),
    ("stinkyscape-060", "Cannon v5", 9, 18),
    ("stinkyscape-051", "Repeater v3", 1, 2),
    ("stinkyscape-055", "Repeater v4", 2, 4),
    ("stinkyscape-059", "Repeater v5", 3, 6),
];
const FLAK_V5: &str = "stinkyscape-061";
/// B20: each Flak and its rounds an activation.
const FLAKS: [(&str, usize); 3] = [(FLAK_V3, 1), ("stinkyscape-057", 2), (FLAK_V5, 3)];
/// B20: the enemies a Flak volley hits each round, p2's two Radiant Big Felinors (6/20, so none dies of a
/// volley); p2's hero too until #2657 took it off the Flaks' rounds.
const ENEMIES_A_ROUND: usize = 2;
const ENEMY_HERO: &str = "hero-p2";

// ---------------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------------

fn filler(count: usize) -> Value {
    json!(vec![FILLER; count])
}

/// The cross-card `scenario`, with R82's auto-end off for both seats, so no step hands the turn over.
fn game(setup: Value) -> Scenario {
    let mut s = scenario(setup);
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

fn def_ids_sorted(cards: &[CardInstance]) -> Vec<String> {
    let mut ids: Vec<String> = cards.iter().map(|card| card.def_id.clone()).collect();
    ids.sort();
    ids
}

fn ids_sorted(ids: &[&str]) -> Vec<String> {
    let mut ids: Vec<String> = ids.iter().map(|id| (*id).to_string()).collect();
    ids.sort();
    ids
}

/// The cards in p1's hand now that `before` did not hold.
fn new_in_hand(before: &[CardInstance], s: &Scenario) -> Vec<CardInstance> {
    s.hand(P1)
        .into_iter()
        .filter(|card| !before.iter().any(|held| held.id == card.id))
        .collect()
}

/// Every `damage` event among `events`, in order, as (target id, amount).
fn hits(events: &[GameEvent]) -> Vec<(String, i32)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Damage {
                target_id, amount, ..
            } => Some((target_id.clone(), *amount)),
            _ => None,
        })
        .collect()
}

/// The def ids every `starshipFused` event among `events` names, in order.
fn fused_into_a_starship(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::StarshipFused { def_id, .. } => Some(def_id.clone()),
            _ => None,
        })
        .collect()
}

/// p1's Dormant Starship, which must exist.
fn p1_ship(s: &Scenario) -> CardInstance {
    must(
        dormant_starship_of(s.state(), P1).cloned(),
        "p1's Dormant Starship",
    )
}

/// The pieces a Starship's record holds, oldest first.
fn pieces_of(ship: &CardInstance) -> Vec<StarshipPieceRecord> {
    ship.starship
        .as_ref()
        .map(|memory| memory.pieces.clone())
        .unwrap_or_default()
}

/// A Starship's record of one piece fused on its base face or its Radiant one.
fn piece(def_id: &str, radiant: bool) -> StarshipPieceRecord {
    StarshipPieceRecord {
        def_id: def_id.to_string(),
        radiant: radiant.then_some(true),
    }
}

/// The catalog ids a definition stands for: its own, or every ingredient's for a fused one.
fn made_of(s: &Scenario, def_id: &str) -> Vec<String> {
    self_def_ids(Some(s.state()), def_id)
}

fn holds(ids: &[String], id: &str) -> bool {
    ids.iter().any(|held| held == id)
}

/// The keyword kinds the card shows through the layers (§10.4).
fn kinds(s: &Scenario, id: &str) -> Vec<KeywordKind> {
    s.stats(id).keywords.iter().map(Keyword::kind).collect()
}

fn in_graveyard(s: &Scenario, player: PlayerId, def_id: &str) -> bool {
    s.pile(player, "graveyard")
        .iter()
        .any(|card| card.def_id == def_id)
}

/// p1's `def` on `radiant`'s face in lane 1 and Big Felinor in hand, which p1 plays into lane 2: its Cry
/// destroys every non-Felinor Unit, a Unit's effect, which Immutable and Immune to Spells let through.
fn felinor_clears(def: &str, radiant: bool) -> Scenario {
    let mut s = game(json!({
        "p1": {
            "field": [{ "def": def, "radiant": radiant, "lane": 1 }],
            "hand": [BIG_FELINOR, ANCHOR],
            "mana": 10,
            "library": filler(3),
        },
        "p2": { "hand": [ANCHOR], "library": filler(3) },
    }));
    s.play(BIG_FELINOR, json!({ "zone": 2 }));
    s
}

/// p1 destroys p1's own lane-1 Unit with Hit Job.
fn hit_job_p1_lane_1(s: &mut Scenario) {
    let target = pick::unit_at(s, P1, 1);
    s.play(HIT_JOB, json!({ "targets": target }));
}

/// SPEC S9: the factions a def id belongs to (its Faction card's, or the Factions that add it).
fn factions_of(def_id: &str) -> Vec<&'static str> {
    FACTIONS
        .iter()
        .filter(|(faction, _, adds)| *faction == def_id || adds.contains(&def_id))
        .map(|(faction, _, _)| *faction)
        .collect()
}

/// B8: whether choosing `chosen` exiles a card of `def_id` from its caster's hand or library.
fn exiled_by_choosing(def_id: &str, chosen: &str) -> bool {
    let of = factions_of(def_id);
    !of.is_empty() && !of.contains(&chosen)
}

/// p1 casts the Swear Loyalities in p1's hand choosing `faction`: the play `legal_actions` lists whose
/// resolution adds that Faction card to p1's hand (R1880002's declared modes, chosen with the play), or,
/// should the play open a prompt, the play and the answer that adds it.
fn swear(s: &mut Scenario, faction: &str) {
    let swear_id = must(
        s.hand(P1).into_iter().find(|card| card.def_id == SWEAR),
        "a Swear Loyalities in p1's hand",
    )
    .id;
    let before: Vec<String> = s.hand(P1).into_iter().map(|card| card.id).collect();
    let adds = |state: &GameState| {
        state.players[P1]
            .hand
            .iter()
            .any(|card| card.def_id == faction && !before.contains(&card.id))
    };
    let mut chosen: Option<(ActionBody, Option<Selection>)> = None;
    'plays: for play in legal_actions(s.state(), P1) {
        if !matches!(&play, ActionBody::Play { instance_id, .. } if *instance_id == swear_id) {
            continue;
        }
        let played = reduce(s.state(), &Action::new(play.clone(), P1, "swear-probe"));
        if played.error.is_some() {
            continue;
        }
        if adds(&played.state) {
            chosen = Some((play, None));
            break;
        }
        let Some(pending) = played.state.pending.clone() else {
            continue;
        };
        for option in pending.options {
            let answer = ActionBody::Answer {
                choice_id: pending.id.clone(),
                selection: vec![option.selection.clone()],
            };
            let answered = reduce(
                &played.state,
                &Action::new(answer, pending.player_id, "swear-answer-probe"),
            );
            if answered.error.is_none() && adds(&answered.state) {
                chosen = Some((play.clone(), Some(option.selection)));
                break 'plays;
            }
        }
    }
    let (play, answer) = must(
        chosen,
        &format!("a listed Swear Loyalities play that chooses {faction}"),
    );
    s.act(play, P1, "cast Swear Loyalities");
    if let Some(selection) = answer {
        let pending = must(s.state().pending.clone(), "Swear Loyalities' choice");
        s.act(
            ActionBody::Answer {
                choice_id: pending.id,
                selection: vec![selection],
            },
            pending.player_id,
            "choose the faction",
        );
    }
}

/// p1's weapon `def` on `radiant`'s face alone in lane 1, against p2's `enemies` (field entries); returns
/// the scenario and the weapon's instance id.
fn armed(def: &str, radiant: bool, enemies: Value) -> (Scenario, String) {
    let s = game(json!({
        "p1": {
            "field": [{ "def": def, "radiant": radiant, "lane": 1 }],
            "hand": [ANCHOR],
            "library": filler(3),
        },
        "p2": { "field": enemies, "hand": [ANCHOR], "library": filler(3) },
    }));
    let weapon = s.unit_at(P1, 1).id;
    (s, weapon)
}

/// The `activate` actions `legal_actions` lists for p1 on the card `card` (an instance id).
fn activations(s: &Scenario, card: &str) -> Vec<ActionBody> {
    legal_actions(s.state(), P1)
        .into_iter()
        .filter(|action| matches!(action, ActionBody::Activate { instance_id, .. } if instance_id == card))
        .collect()
}

/// The ability the listed `activate` of `card` names.
fn ability_of(s: &Scenario, card: &str) -> String {
    must(
        activations(s, card).into_iter().find_map(|action| match action {
            ActionBody::Activate { ability, .. } => ability,
            _ => None,
        }),
        &format!("a listed `activate` of {card} naming its ability"),
    )
}

/// `card`'s `ability` aimed at `target`, as a client sends it.
fn aimed(card: &str, ability: &str, target: Selection) -> ActionBody {
    ActionBody::Activate {
        instance_id: card.to_string(),
        ability: Some(ability.to_string()),
        targets: Some(vec![target]),
        modes: None,
        tributes: None,
    }
}

fn the_card(instance_id: &str) -> Selection {
    Selection::Instance {
        instance_id: instance_id.to_string(),
    }
}

fn aims_at_a_hero(action: &ActionBody) -> bool {
    targets_of(action)
        .iter()
        .any(|target| matches!(target, Selection::Hero { .. }))
}

/// B20: what one Flak activation did.
struct Volley {
    /// Every hit of the activation, in order, as (target id, amount).
    hits: Vec<(String, i32)>,
    /// p2's two Big Felinors' instance ids, sorted.
    units: Vec<String>,
}

/// A Flak left as printed.
fn untuned(_: &mut CardInstance) {}

/// B20: one activation of p1's Flak `def` on `radiant`'s face, `tune` applied to it first, against p2's
/// two Radiant Big Felinors and hero, from `seed`.
fn volley(def: &str, radiant: bool, seed: &str, tune: &dyn Fn(&mut CardInstance)) -> Volley {
    let mut s = game(json!({
        "seed": seed,
        "p1": {
            "field": [{ "def": def, "radiant": radiant, "lane": 1 }],
            "hand": [ANCHOR],
            "library": filler(3),
        },
        "p2": {
            "field": [
                { "def": BIG_FELINOR, "radiant": true, "lane": 1 },
                { "def": BIG_FELINOR, "radiant": true, "lane": 2 },
            ],
            "hand": [ANCHOR],
            "library": filler(3),
        },
    }));
    let flak = s.unit_at(P1, 1).id;
    tune(s.card_mut(flak.as_str()));
    let mut units = vec![s.unit_at(P2, 1).id, s.unit_at(P2, 2).id];
    units.sort();
    let activation = must(
        activations(&s, &flak).into_iter().next(),
        &format!("a listed `activate` of {def}"),
    );
    s.act(activation, P1, "activate the Flak");
    Volley {
        hits: hits(s.last_events()),
        units,
    }
}

/// Every amount of `seeds` volleys of `def`.
fn amounts(
    def: &str,
    radiant: bool,
    prefix: &str,
    seeds: usize,
    tune: &dyn Fn(&mut CardInstance),
) -> Vec<i32> {
    (0..seeds)
        .flat_map(|i| {
            volley(def, radiant, &format!("{prefix}-{i}"), tune)
                .hits
                .into_iter()
                .map(|(_, amount)| amount)
        })
        .collect()
}

fn tally(amounts: &[i32], value: i32) -> usize {
    amounts.iter().filter(|amount| **amount == value).count()
}

// ---------------------------------------------------------------------------------------------------
// B1–B5: the Resources
// ---------------------------------------------------------------------------------------------------

#[test]
fn b1_reknite_korrelite_gellium_axnit_narcor_and_red_narcor_carry_resource_and_token() {
    for id in RESOURCES {
        assert_eq!(card_def(id).tags, vec![Tag::Resource, Tag::Token], "{id}");
    }
}

#[test]
fn b2_korrelite_prices_its_credits_2_to_5_and_radiant_4_to_10_and_a_card_with_no_resource_prices_none() {
    let korrelite = serde_json::to_value(card_def(KORRELITE)).expect("a CardDef serializes");
    let prices = &korrelite["resource"];
    assert_eq!(
        (&prices["base"]["sellMin"], &prices["base"]["sellMax"]),
        (&json!(2), &json!(5)),
        "{prices}"
    );
    assert_eq!(
        (&prices["radiant"]["sellMin"], &prices["radiant"]["sellMax"]),
        (&json!(4), &json!(10)),
        "{prices}"
    );
    assert!(card_def(VANILLA).resource.is_none());
}

#[test]
fn b3_base_korrelite_deals_exactly_one_hit_of_1_to_an_armored_hero_through_its_armor() {
    let mut s = game(json!({
        "p1": { "hand": [KORRELITE, ANCHOR], "library": filler(3) },
        "p2": { "armor": 3, "hand": [ANCHOR], "library": filler(3) },
    }));
    s.play(KORRELITE, json!({}));
    assert_eq!(
        hits(s.events()),
        vec![(ENEMY_HERO.to_string(), 1)],
        "Pierce: Armor 3 takes nothing off; the base face has no Echo"
    );
    s.expect_health(P2, 29);
}

#[test]
fn b3_korrelite_pierces_an_armor_7_unit_and_an_armored_hero_alike() {
    let mut unit_hit = false;
    for i in 0..30 {
        let mut s = game(json!({
            "seed": format!("b3-korrelite-armor-{i}"),
            "p1": { "hand": [KORRELITE, ANCHOR], "library": filler(3) },
            "p2": { "field": [SEVEN], "armor": 3, "hand": [ANCHOR], "library": filler(3) },
        }));
        let seven = s.unit_at(P2, 1).id;
        s.play(KORRELITE, json!({}));
        let dealt = hits(s.events());
        assert_eq!(dealt.len(), 1, "seed {i}: one hit, {dealt:?}");
        let (target, amount) = &dealt[0];
        assert_eq!(*amount, 1, "seed {i}: Pierce, so no Armor takes anything off");
        if *target == seven {
            unit_hit = true;
            s.expect_stats(seven.as_str(), json!({ "health": 6 }));
        } else {
            assert_eq!(target, ENEMY_HERO, "seed {i}: Korrelite hits an enemy");
            s.expect_health(P2, 29);
        }
    }
    assert!(
        unit_hit,
        "in 30 seeds the random enemy was the Armor 7 Unit at least once"
    );
}

#[test]
fn b3_radiant_korrelite_echoes_once_for_two_pierced_hits_in_all() {
    let mut s = game(json!({
        "p1": { "hand": [{ "def": KORRELITE, "radiant": true }, ANCHOR], "library": filler(3) },
        "p2": { "armor": 3, "hand": [ANCHOR], "library": filler(3) },
    }));
    s.play(KORRELITE, json!({}));
    assert_eq!(
        hits(s.events()),
        vec![(ENEMY_HERO.to_string(), 1), (ENEMY_HERO.to_string(), 1)],
        "Echo 1: twice in all, each through Armor 3"
    );
    s.expect_health(P2, 28);
}

/// B4: Narcor on `radiant`'s face adds one card, every time a Legendary non-token of a set that ships on
/// that face; across the plays it is no fixed card.
fn narcor_adds_one_legendary(radiant: bool) {
    let mut seen: IndexSet<String> = IndexSet::new();
    for i in 0..NARCOR_PLAYS {
        let mut s = game(json!({
            "seed": format!("b4-narcor-{radiant}-{i}"),
            "p1": { "hand": [{ "def": NARCOR, "radiant": radiant }, ANCHOR], "library": filler(3) },
            "p2": { "hand": [ANCHOR], "library": filler(3) },
        }));
        let before = s.hand(P1);
        s.play(NARCOR, json!({}));
        let added = new_in_hand(&before, &s);
        assert_eq!(
            added.len(),
            1,
            "seed {i}: Narcor adds one card, not {:?}",
            def_ids_sorted(&added)
        );
        let card = &added[0];
        let def = card_def(&card.def_id);
        assert_eq!(
            def.rarity,
            Rarity::Legendary,
            "seed {i}: {} is no Legendary",
            def.id
        );
        assert!(!def.token, "seed {i}: {} is a token", def.id);
        assert!(
            set_is_open(def.set),
            "seed {i}: {} is of a set that does not ship",
            def.id
        );
        assert_eq!(card.radiant, radiant, "seed {i}: {} on the wrong face", def.id);
        seen.insert(def.id);
    }
    assert!(
        seen.len() > 1,
        "a random Legendary card, not one fixed card: {seen:?}"
    );
}

#[test]
fn b4_narcor_never_adds_a_mythic_a_token_or_a_card_of_a_set_that_does_not_ship_over_many_seeds() {
    narcor_adds_one_legendary(false);
}

#[test]
fn b4_radiant_narcor_adds_its_one_random_legendary_on_its_radiant_face_over_many_seeds() {
    narcor_adds_one_legendary(true);
}

/// B5: Red Narcor on `radiant`'s face from 1 mana and a 4-card library; returns the scenario and p1's
/// max mana before the play.
fn red_narcor(radiant: bool) -> (Scenario, i32) {
    let mut s = game(json!({
        "p1": {
            "hand": [{ "def": RED_NARCOR, "radiant": radiant }, ANCHOR],
            "library": filler(4),
            "mana": 1,
        },
        "p2": { "hand": [ANCHOR], "library": filler(3) },
    }));
    let max = s.state().players[P1].mana.max;
    s.play(RED_NARCOR, json!({}));
    (s, max)
}

#[test]
fn b5_red_narcor_gains_its_caster_2_mana_and_draws_1() {
    let (mut s, _) = red_narcor(false);
    s.expect_mana(P1, 3);
    assert_eq!(s.hand(P1).len(), 2, "the anchor and one card drawn");
    assert_eq!(s.pile(P1, "library").len(), 3);
}

#[test]
fn b5_radiant_red_narcor_gains_its_caster_4_mana_and_draws_2() {
    let (mut s, _) = red_narcor(true);
    s.expect_mana(P1, 5);
    assert_eq!(s.hand(P1).len(), 3, "the anchor and two cards drawn");
    assert_eq!(s.pile(P1, "library").len(), 2);
}

#[test]
fn b5_red_narcor_s_mana_is_this_turn_s_and_leaves_the_max_where_it_was_even_past_it() {
    for radiant in [false, true] {
        let (s, max) = red_narcor(radiant);
        let mana = s.state().players[P1].mana;
        let gained = if radiant { 4 } else { 2 };
        assert_eq!(
            mana.max, max,
            "radiant {radiant}: Gain mana adds to this turn's mana only"
        );
        assert_eq!(
            mana.current,
            1 + gained,
            "radiant {radiant}: past the max of {max}"
        );
    }
}

// ---------------------------------------------------------------------------------------------------
// B6–B8: the Factions and Swear Loyalities
// ---------------------------------------------------------------------------------------------------

#[test]
fn b6_the_five_faction_cards_are_named_faction_colon_tagged_faction_and_cost_2() {
    for (id, name, _) in FACTIONS {
        let def = card_def(id);
        assert_eq!(def.name, name, "{id}");
        assert_eq!(def.tags, vec![Tag::Faction], "{id}");
        assert_eq!(query_cost(&def), 2, "{id}");
    }
}

#[test]
fn b6_each_faction_card_still_adds_its_cards_on_the_face_it_was_cast_on() {
    for (id, name, adds) in FACTIONS {
        for radiant in [false, true] {
            let mut s = game(json!({
                "p1": {
                    "hand": [{ "def": id, "radiant": radiant }, ANCHOR],
                    "mana": 2,
                    "library": filler(3),
                },
                "p2": { "hand": [ANCHOR], "library": filler(3) },
            }));
            let before = s.hand(P1);
            s.play(id, json!({}));
            let added = new_in_hand(&before, &s);
            assert_eq!(
                def_ids_sorted(&added),
                ids_sorted(adds),
                "{name}, radiant {radiant}"
            );
            assert!(
                added.iter().all(|card| card.radiant == radiant),
                "{name}: every card it adds is on its face (radiant {radiant})"
            );
        }
    }
}

#[test]
fn b6_a_faction_card_is_refused_at_1_mana() {
    for (id, name, _) in FACTIONS {
        let mut s = game(json!({
            "p1": { "hand": [id, ANCHOR], "mana": 1, "library": filler(3) },
            "p2": { "hand": [ANCHOR], "library": filler(3) },
        }));
        s.expect_refused_with(|s| s.play(id, json!({})), "costs 2");
        assert!(
            s.hand(P1).iter().any(|card| card.def_id == id),
            "{name} stays in hand"
        );
    }
}

#[test]
fn b7_swear_loyalities_costs_0_and_is_cast_at_0_mana() {
    assert_eq!(query_cost(&card_def(SWEAR)), 0);
    let mut s = game(json!({
        "p1": { "hand": [SWEAR, ANCHOR], "mana": 0, "library": filler(3) },
        "p2": { "hand": [ANCHOR], "library": filler(3) },
    }));
    swear(&mut s, PEGASUS);
    s.expect_mana(P1, 0);
    assert!(s.hand(P1).iter().any(|card| card.def_id == PEGASUS));
}

#[test]
fn b7_swear_loyalities_adds_the_chosen_faction_card_of_all_five_on_its_own_face() {
    for (faction, name, _) in FACTIONS {
        for radiant in [false, true] {
            let mut s = game(json!({
                "p1": { "hand": [{ "def": SWEAR, "radiant": radiant }, ANCHOR], "library": filler(3) },
                "p2": { "hand": [ANCHOR], "library": filler(3) },
            }));
            let before = s.hand(P1);
            swear(&mut s, faction);
            let added = new_in_hand(&before, &s);
            assert_eq!(
                def_ids_sorted(&added),
                vec![faction.to_string()],
                "{name}, radiant {radiant}"
            );
            assert_eq!(
                added[0].radiant, radiant,
                "{name}: the Radiant Swear Loyalities adds the Radiant face, the base one the base face"
            );
        }
    }
}

#[test]
fn b8_swear_loyalities_exiles_every_card_of_another_faction_from_its_caster_s_hand_and_library() {
    let hand = [
        VANILLA,
        "stinkyscape-111",
        "stinkyscape-096",
        "stinkyscape-073",
        "stinkyscape-123",
        "stinkyscape-076",
    ];
    let library = [
        FILLER,
        "stinkyscape-109",
        "stinkyscape-103",
        "stinkyscape-069",
        "stinkyscape-114",
        "stinkyscape-080",
        STAR_JUMPER,
        ETHICS,
    ];
    for (chosen, name, _) in FACTIONS {
        let mut held = vec![SWEAR];
        held.extend(hand);
        let mut s = game(json!({
            "p1": { "hand": held, "library": library },
            "p2": { "hand": [ANCHOR], "library": filler(3) },
        }));
        swear(&mut s, chosen);

        let exiled: Vec<&str> = hand
            .iter()
            .chain(library.iter())
            .copied()
            .filter(|id| exiled_by_choosing(id, chosen))
            .collect();
        let kept_hand: Vec<&str> = hand
            .iter()
            .copied()
            .filter(|id| !exiled_by_choosing(id, chosen))
            .chain([chosen])
            .collect();
        let kept_library: Vec<&str> = library
            .iter()
            .copied()
            .filter(|id| !exiled_by_choosing(id, chosen))
            .collect();
        assert!(
            exiled.len() >= 8,
            "the fixture holds other factions' cards for {name}"
        );
        assert_eq!(
            def_ids_sorted(&s.pile(P1, "exile")),
            ids_sorted(&exiled),
            "choosing {name}"
        );
        assert_eq!(
            def_ids_sorted(&s.hand(P1)),
            ids_sorted(&kept_hand),
            "choosing {name}"
        );
        assert_eq!(
            def_ids_sorted(&s.pile(P1, "library")),
            ids_sorted(&kept_library),
            "choosing {name}"
        );
    }
}

#[test]
fn b8_a_card_of_no_faction_is_never_exiled_whichever_faction_is_chosen() {
    let hand = [VANILLA, MENACE, STRATOS, ANCHOR];
    let library = [FILLER, FLAK_V3, SEVEN, CLOAKING_MODULE];
    for (chosen, name, _) in FACTIONS {
        let mut held = vec![SWEAR];
        held.extend(hand);
        let mut s = game(json!({
            "p1": { "hand": held, "library": library },
            "p2": { "hand": [ANCHOR], "library": filler(3) },
        }));
        swear(&mut s, chosen);
        let exiled = def_ids_sorted(&s.pile(P1, "exile"));
        assert!(exiled.is_empty(), "choosing {name}: {exiled:?}");
        let mut kept_hand = hand.to_vec();
        kept_hand.push(chosen);
        assert_eq!(
            def_ids_sorted(&s.hand(P1)),
            ids_sorted(&kept_hand),
            "choosing {name}"
        );
        assert_eq!(
            def_ids_sorted(&s.pile(P1, "library")),
            ids_sorted(&library),
            "choosing {name}"
        );
    }
}

#[test]
fn b8_tylannt_stays_when_ethics_or_vanguard_is_chosen_and_is_exiled_otherwise() {
    for (chosen, name, _) in FACTIONS {
        let mut s = game(json!({
            "p1": { "hand": [SWEAR, TYLANNT, ANCHOR], "library": [TYLANNT, FILLER, FILLER] },
            "p2": { "hand": [ANCHOR], "library": filler(3) },
        }));
        swear(&mut s, chosen);
        let count = |cards: Vec<CardInstance>| cards.iter().filter(|card| card.def_id == TYLANNT).count();
        let (in_hand, in_library, exiled) = (
            count(s.hand(P1)),
            count(s.pile(P1, "library")),
            count(s.pile(P1, "exile")),
        );
        let expected = if chosen == ETHICS || chosen == VANGUARD {
            (1, 1, 0)
        } else {
            (0, 0, 2)
        };
        assert_eq!(
            (in_hand, in_library, exiled),
            expected,
            "choosing {name}: Tylannt is of ETHICS and of Vanguard Coalition"
        );
    }
}

#[test]
fn b8_graveyards_the_field_and_the_other_player_s_zones_are_never_exiled() {
    let p1_graveyard = [
        "stinkyscape-110",
        "stinkyscape-072",
        "stinkyscape-115",
        "stinkyscape-076",
    ];
    let p2_hand = ["stinkyscape-115", "stinkyscape-074", "stinkyscape-113", ANCHOR];
    let p2_library = ["stinkyscape-094", "stinkyscape-078", "stinkyscape-110"];
    let p2_graveyard = ["stinkyscape-096", "stinkyscape-161"];
    for (chosen, name, _) in FACTIONS {
        let mut s = game(json!({
            "p1": {
                "hand": [SWEAR, ANCHOR],
                "library": filler(2),
                "graveyard": p1_graveyard,
                "field": [{ "def": "stinkyscape-079", "lane": 1 }, { "def": "stinkyscape-094", "lane": 2 }],
            },
            "p2": { "hand": p2_hand, "library": p2_library, "graveyard": p2_graveyard },
        }));
        swear(&mut s, chosen);

        assert!(
            s.pile(P1, "exile").is_empty(),
            "choosing {name}: p1 held no other faction's card in hand or library"
        );
        let p1_graveyard_now: Vec<CardInstance> = s
            .pile(P1, "graveyard")
            .into_iter()
            .filter(|card| card.def_id != SWEAR)
            .collect();
        assert_eq!(
            def_ids_sorted(&p1_graveyard_now),
            ids_sorted(&p1_graveyard),
            "choosing {name}"
        );
        assert_eq!(s.unit_at(P1, 1).def_id, "stinkyscape-079", "choosing {name}");
        assert_eq!(s.unit_at(P1, 2).def_id, "stinkyscape-094", "choosing {name}");
        assert_eq!(
            def_ids_sorted(&s.hand(P2)),
            ids_sorted(&p2_hand),
            "choosing {name}"
        );
        assert_eq!(
            def_ids_sorted(&s.pile(P2, "library")),
            ids_sorted(&p2_library),
            "choosing {name}"
        );
        assert_eq!(
            def_ids_sorted(&s.pile(P2, "graveyard")),
            ids_sorted(&p2_graveyard),
            "choosing {name}"
        );
        assert!(s.pile(P2, "exile").is_empty(), "choosing {name}");
    }
}

#[test]
fn b8_the_faction_card_just_added_and_copies_of_it_stay_while_other_faction_cards_go() {
    let mut s = game(json!({
        "p1": { "hand": [SWEAR, ETHICS, ANCHOR], "library": [ETHICS, GOLDEN, PEGASUS, FILLER] },
        "p2": { "hand": [ANCHOR], "library": filler(3) },
    }));
    swear(&mut s, ETHICS);
    let ethics_in = |cards: Vec<CardInstance>| cards.iter().filter(|card| card.def_id == ETHICS).count();
    assert_eq!(ethics_in(s.hand(P1)), 2, "the copy held and the one just added");
    assert_eq!(ethics_in(s.pile(P1, "library")), 1);
    assert_eq!(
        def_ids_sorted(&s.pile(P1, "exile")),
        ids_sorted(&[GOLDEN, PEGASUS])
    );
}

// ---------------------------------------------------------------------------------------------------
// B9–B16: the Starship Pieces
// ---------------------------------------------------------------------------------------------------

#[test]
fn b9_the_fifteen_die_on_either_face_and_fuse_nothing() {
    for def in NOT_PIECES {
        for radiant in [false, true] {
            let s = felinor_clears(def, radiant);
            assert!(in_graveyard(&s, P1, def), "{def} (radiant {radiant}) died");
            let fused = fused_into_a_starship(s.events());
            assert!(
                fused.is_empty(),
                "{def} (radiant {radiant}) is no Starship Piece: {fused:?}"
            );
            assert!(
                dormant_starship_of(s.state(), P1).is_none(),
                "{def} (radiant {radiant}) built p1 no Dormant Starship"
            );
        }
    }
}

#[test]
fn b9_a_starship_piece_killed_the_same_way_still_fuses() {
    let s = felinor_clears(FLAK_V3, false);
    assert_eq!(fused_into_a_starship(s.events()), vec![FLAK_V3.to_string()]);
    assert_eq!(pieces_of(&p1_ship(&s)), vec![piece(FLAK_V3, false)]);
}

#[test]
fn b9_the_random_starship_piece_pool_offers_none_of_the_fifteen() {
    jackioh_cards::register_all();
    let pool: Vec<String> = jackioh_cards::query(&jackioh_cards::starship_pieces())
        .iter()
        .map(|def| def.id.clone())
        .collect();
    for def in NOT_PIECES {
        assert!(
            !holds(&pool, def),
            "{def} is offered as a Starship Piece: {pool:?}"
        );
    }
    assert!(holds(&pool, FLAK_V3), "{pool:?}");
    assert!(holds(&pool, CLOAKING_MODULE), "{pool:?}");
}

#[test]
fn b10_a_flak_v3_with_a_card_fused_into_it_fuses_flak_v3_alone_into_the_starship() {
    let mut s = game(json!({
        "p1": {
            "field": [{ "def": FLAK_V3, "lane": 1 }],
            "hand": [CAT_EARS, HIT_JOB, ANCHOR],
            "mana": 10,
            "library": filler(3),
        },
        "p2": { "hand": [ANCHOR], "library": filler(3) },
    }));
    s.play(CAT_EARS, json!({ "zone": 1, "equip": true }));
    let host = made_of(&s, &s.unit_at(P1, 1).def_id);
    assert!(
        holds(&host, FLAK_V3) && holds(&host, CAT_EARS),
        "setup: Sentient Cat Ears is fused into the Flak v3: {host:?}"
    );

    hit_job_p1_lane_1(&mut s);

    assert_eq!(fused_into_a_starship(s.events()), vec![FLAK_V3.to_string()]);
    let ship = p1_ship(&s);
    assert_eq!(pieces_of(&ship), vec![piece(FLAK_V3, false)]);
    let ship_made_of = made_of(&s, &ship.def_id);
    assert!(holds(&ship_made_of, FLAK_V3), "{ship_made_of:?}");
    assert!(
        !holds(&ship_made_of, CAT_EARS),
        "the Starship gains only Flak v3's text: {ship_made_of:?}"
    );
}

#[test]
fn b11_a_buffed_piece_fuses_with_its_printed_numbers_not_its_buffed_ones() {
    for radiant in [false, true] {
        let mut s = game(json!({
            "p1": {
                "field": [{ "def": CANNON_V3, "radiant": radiant, "lane": 1 }],
                "hand": [HIT_JOB, ANCHOR],
                "mana": 10,
                "library": filler(3),
            },
            "p2": { "hand": [ANCHOR], "library": filler(3) },
        }));
        let cannon = s.unit_at(P1, 1).id;
        step_param(s.card_mut(cannon.as_str()), "damage", 2);
        let printed = if radiant { 10 } else { 5 };
        let buffed = params_view(s.state(), s.card(cannon.as_str()))
            .and_then(|numbers| numbers.get("damage").copied());
        assert!(
            buffed.is_some_and(|damage| damage > printed),
            "setup: the Cannon v3 is Buffed past {printed}: {buffed:?}"
        );

        hit_job_p1_lane_1(&mut s);

        assert_eq!(
            fused_into_a_starship(s.events()),
            vec![CANNON_V3.to_string()],
            "radiant {radiant}"
        );
        let ship = p1_ship(&s);
        assert_eq!(pieces_of(&ship), vec![piece(CANNON_V3, radiant)]);
        let steps = ship
            .tuning
            .as_ref()
            .and_then(|tuning| tuning.numbers.as_ref())
            .and_then(|numbers| numbers.get("damage").copied())
            .unwrap_or(0);
        assert_eq!(steps, 0, "radiant {radiant}: the Buff stays with the dead card");
        if !radiant {
            assert_eq!(
                params_view(s.state(), &ship).and_then(|numbers| numbers.get("damage").copied()),
                Some(5),
                "the Starship's Cannon v3 deals its printed 5"
            );
        }
    }
}

#[test]
fn b12_a_radiant_piece_fuses_on_its_radiant_face() {
    let mut s = game(json!({
        "p1": {
            "field": [{ "def": CANNON_V3, "radiant": true, "lane": 1 }],
            "hand": [HIT_JOB, ANCHOR],
            "mana": 10,
            "library": filler(3),
        },
        "p2": { "hand": [ANCHOR], "library": filler(3) },
    }));
    hit_job_p1_lane_1(&mut s);
    assert_eq!(fused_into_a_starship(s.events()), vec![CANNON_V3.to_string()]);
    assert_eq!(pieces_of(&p1_ship(&s)), vec![piece(CANNON_V3, true)]);
}

#[test]
fn b12_a_base_piece_never_fuses_on_the_radiant_face() {
    let mut s = game(json!({
        "p1": {
            "field": [{ "def": CANNON_V3, "lane": 1 }],
            "hand": [HIT_JOB, ANCHOR],
            "mana": 10,
            "library": filler(3),
        },
        "p2": { "hand": [ANCHOR], "library": filler(3) },
    }));
    hit_job_p1_lane_1(&mut s);
    assert_eq!(pieces_of(&p1_ship(&s)), vec![piece(CANNON_V3, false)]);
}

#[test]
fn b13_stratos_is_3_1_and_radiant_6_2_with_flying_and_ranged() {
    let s = game(json!({
        "p1": {
            "field": [{ "def": STRATOS, "lane": 1 }, { "def": STRATOS, "radiant": true, "lane": 2 }],
            "hand": [ANCHOR],
            "library": filler(3),
        },
        "p2": { "hand": [ANCHOR], "library": filler(3) },
    }));
    for (lane, stats) in [(1, (3, 1)), (2, (6, 2))] {
        let stratos = s.unit_at(P1, lane).id;
        let view = s.stats(stratos.as_str());
        assert_eq!((view.attack, view.max_health), stats, "lane {lane}");
        let shown = kinds(&s, &stratos);
        assert!(
            shown.contains(&KeywordKind::Flying) && shown.contains(&KeywordKind::Ranged),
            "lane {lane}: {shown:?}"
        );
    }
}

/// B13: p1 casts Stratos Swarm on `radiant`'s face over p1's `field`.
fn swarm(radiant: bool, field: Value) -> Scenario {
    let mut s = game(json!({
        "p1": {
            "hand": [{ "def": STRATOS_SWARM, "radiant": radiant }, ANCHOR],
            "field": field,
            "mana": 10,
            "library": filler(3),
        },
        "p2": { "hand": [ANCHOR], "library": filler(3) },
    }));
    s.play(STRATOS_SWARM, json!({}));
    s
}

#[test]
fn b13_stratos_swarm_fills_every_unit_zone_with_stratos_on_its_own_face() {
    for radiant in [false, true] {
        let s = swarm(radiant, json!([]));
        for lane in 1..=row_size(Row::Units) {
            let unit = s.unit_at(P1, lane);
            assert_eq!(unit.def_id, STRATOS, "radiant {radiant}, lane {lane}");
            assert_eq!(unit.radiant, radiant, "lane {lane}");
            let view = s.stats(unit.id.as_str());
            let stats = if radiant { (6, 2) } else { (3, 1) };
            assert_eq!(
                (view.attack, view.max_health),
                stats,
                "radiant {radiant}, lane {lane}"
            );
            let shown = kinds(&s, &unit.id);
            assert!(
                shown.contains(&KeywordKind::Flying) && shown.contains(&KeywordKind::Ranged),
                "radiant {radiant}, lane {lane}: {shown:?}"
            );
        }
    }
}

#[test]
fn b13_stratos_swarm_leaves_an_occupied_unit_zone_alone() {
    let mut s = game(json!({
        "p1": {
            "hand": [STRATOS_SWARM, ANCHOR],
            "field": [{ "def": VANILLA, "lane": 3 }],
            "mana": 10,
            "library": filler(3),
        },
        "p2": { "hand": [ANCHOR], "library": filler(3) },
    }));
    let vanilla = s.unit_at(P1, 3).id;
    s.play(STRATOS_SWARM, json!({}));
    assert_eq!(s.unit_at(P1, 3).id, vanilla, "the Vanilla keeps its zone");
    for lane in (1..=row_size(Row::Units)).filter(|lane| *lane != 3) {
        assert_eq!(s.unit_at(P1, lane).def_id, STRATOS, "lane {lane}");
    }
}

#[test]
fn b14_cloaking_module_is_a_2_cost_2_2_rare_unit_whose_radiant_4_4_adds_immune_to_spells() {
    let def = card_def(CLOAKING_MODULE);
    assert_eq!(def.type_, CardType::Unit);
    assert_eq!(def.rarity, Rarity::Rare);
    assert_eq!(query_cost(&def), 2);
    let printed = vec![
        KeywordKind::StarshipPiece,
        KeywordKind::Equip,
        KeywordKind::Cloaked,
        KeywordKind::Immutable,
    ];
    let base: Vec<KeywordKind> = def.base.keywords.iter().map(Keyword::kind).collect();
    let radiant: Vec<KeywordKind> = def.radiant.keywords.iter().map(Keyword::kind).collect();
    assert_eq!(base, printed);
    let mut radiant_printed = printed;
    radiant_printed.push(KeywordKind::ImmuneToSpells);
    assert_eq!(radiant, radiant_printed);
    let faces = serde_json::to_value(def).expect("a CardDef serializes");
    assert_eq!(
        (&faces["base"]["attack"], &faces["base"]["health"]),
        (&json!(2), &json!(2))
    );
    assert_eq!(
        (&faces["radiant"]["attack"], &faces["radiant"]["health"]),
        (&json!(4), &json!(4))
    );
}

#[test]
fn b14_cloaking_module_is_played_onto_a_unit_by_its_equip() {
    for radiant in [false, true] {
        let mut s = game(json!({
            "p1": {
                "field": [{ "def": VANILLA, "lane": 1 }],
                "hand": [{ "def": CLOAKING_MODULE, "radiant": radiant }, ANCHOR],
                "mana": 10,
                "library": filler(3),
            },
            "p2": { "hand": [ANCHOR], "library": filler(3) },
        }));
        s.play(CLOAKING_MODULE, json!({ "zone": 1, "equip": true }));
        let host = made_of(&s, &s.unit_at(P1, 1).def_id);
        assert!(
            holds(&host, CLOAKING_MODULE) && holds(&host, VANILLA),
            "radiant {radiant}: fused into the Vanilla: {host:?}"
        );
        assert!(
            s.unit(P1, 2).is_none(),
            "radiant {radiant}: it took no zone of its own"
        );
        assert!(!s.hand(P1).iter().any(|card| card.def_id == CLOAKING_MODULE));
    }
}

#[test]
fn b14_cloaking_module_is_set_face_down_by_its_cloaked() {
    for radiant in [false, true] {
        let mut s = game(json!({
            "p1": {
                "hand": [{ "def": CLOAKING_MODULE, "radiant": radiant }, ANCHOR],
                "mana": 10,
                "library": filler(3),
            },
            "p2": { "hand": [ANCHOR], "library": filler(3) },
        }));
        s.play(
            CLOAKING_MODULE,
            json!({ "zone": 1, "row": "backrow", "faceDown": "cloaked" }),
        );
        let set = must(s.backrow(P1, 1), "the set Cloaking Module");
        assert_eq!(set.def_id, CLOAKING_MODULE, "radiant {radiant}");
        assert_ne!(set.face_up, Some(true), "radiant {radiant}: set face-down");
        assert!(
            s.unit(P1, 1).is_none(),
            "radiant {radiant}: no Unit stands in lane 1"
        );
    }
}

#[test]
fn b14_cloaking_module_dies_as_a_starship_piece_on_either_face() {
    for radiant in [false, true] {
        let s = felinor_clears(CLOAKING_MODULE, radiant);
        assert!(in_graveyard(&s, P1, CLOAKING_MODULE), "radiant {radiant}");
        assert_eq!(
            fused_into_a_starship(s.events()),
            vec![CLOAKING_MODULE.to_string()]
        );
        assert_eq!(pieces_of(&p1_ship(&s)), vec![piece(CLOAKING_MODULE, radiant)]);
    }
}

#[test]
fn b14_a_spell_destroying_all_permanents_takes_the_base_module_but_never_the_radiant_one() {
    for radiant in [false, true] {
        let mut s = game(json!({
            "p1": {
                "field": [{ "def": CLOAKING_MODULE, "radiant": radiant, "lane": 1 }],
                "hand": [TWISTING_NETHER, ANCHOR],
                "mana": 10,
                "library": filler(3),
            },
            "p2": { "hand": [ANCHOR], "library": filler(3) },
        }));
        let module = s.unit_at(P1, 1).id;
        s.play(TWISTING_NETHER, json!({}));
        if radiant {
            s.expect_in_zone(module.as_str(), "field");
            assert!(
                dormant_starship_of(s.state(), P1).is_none(),
                "nothing died to fuse"
            );
        } else {
            assert!(
                in_graveyard(&s, P1, CLOAKING_MODULE),
                "Immutable keeps no \"all\" effect off it"
            );
            assert_eq!(pieces_of(&p1_ship(&s)), vec![piece(CLOAKING_MODULE, false)]);
        }
    }
}

#[test]
fn b15_paragon_x_survives_a_spell_that_destroys_all_permanents_on_either_face() {
    for radiant in [false, true] {
        let mut s = game(json!({
            "p1": { "hand": [TWISTING_NETHER, ANCHOR], "mana": 10, "library": filler(3) },
            "p2": {
                "field": [{ "def": PARAGON_X, "radiant": radiant, "lane": 1 }, { "def": VANILLA, "lane": 2 }],
                "hand": [ANCHOR],
                "library": filler(3),
            },
        }));
        let paragon = s.unit_at(P2, 1).id;
        s.play(TWISTING_NETHER, json!({}));
        s.expect_in_zone(paragon.as_str(), "field");
        assert!(
            in_graveyard(&s, P2, VANILLA),
            "radiant {radiant}: the Spell did resolve"
        );
    }
}

#[test]
fn b15_no_spell_may_target_paragon_x() {
    for radiant in [false, true] {
        let mut s = game(json!({
            "p1": { "hand": [HIT_JOB, ANCHOR], "mana": 10, "library": filler(3) },
            "p2": {
                "field": [{ "def": PARAGON_X, "radiant": radiant, "lane": 1 }, { "def": VANILLA, "lane": 2 }],
                "hand": [ANCHOR],
                "library": filler(3),
            },
        }));
        let paragon = s.unit_at(P2, 1).id;
        let vanilla = s.unit_at(P2, 2).id;
        let hit_job = s.card(HIT_JOB).id.clone();
        let plays: Vec<ActionBody> = legal_actions(s.state(), P1)
            .into_iter()
            .filter(
                |action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == hit_job),
            )
            .collect();
        assert!(
            plays
                .iter()
                .any(|play| targets_of(play) == vec![the_card(&vanilla)]),
            "radiant {radiant}: Hit Job may aim at the Vanilla"
        );
        assert!(
            !plays
                .iter()
                .any(|play| targets_of(play).contains(&the_card(&paragon))),
            "radiant {radiant}: Hit Job is offered Paragon-X"
        );
        let at_paragon = pick::id(&paragon);
        s.expect_refused(|s| s.play(HIT_JOB, json!({ "targets": at_paragon })));
        s.expect_in_zone(paragon.as_str(), "field");
    }
}

#[test]
fn b15_paragon_x_shows_immune_to_spells_on_either_face() {
    let s = game(json!({
        "p1": {
            "field": [{ "def": PARAGON_X, "lane": 1 }, { "def": PARAGON_X, "radiant": true, "lane": 2 }],
            "hand": [ANCHOR],
            "library": filler(3),
        },
        "p2": { "hand": [ANCHOR], "library": filler(3) },
    }));
    for lane in [1, 2] {
        let paragon = s.unit_at(P1, lane).id;
        assert!(
            kinds(&s, &paragon).contains(&KeywordKind::ImmuneToSpells),
            "lane {lane}: {:?}",
            kinds(&s, &paragon)
        );
    }
}

/// B16: p1 plays Jlockeed's Yamato Cannon on `radiant`'s face into lane 1, p2's Vanilla across; returns
/// the scenario and the Cannon's instance id.
fn yamato_played(radiant: bool) -> (Scenario, String) {
    let mut s = game(json!({
        "p1": { "hand": [{ "def": YAMATO, "radiant": radiant }, ANCHOR], "mana": 10, "library": filler(3) },
        "p2": { "field": [VANILLA], "hand": [ANCHOR], "library": filler(3) },
    }));
    s.play(YAMATO, json!({ "zone": 1 }));
    let yamato = s.unit_at(P1, 1).id;
    (s, yamato)
}

#[test]
fn b16_radiant_yamato_cannon_attacks_a_unit_the_turn_it_is_played() {
    let (mut s, yamato) = yamato_played(true);
    let vanilla = s.unit_at(P2, 1).id;
    s.attack(yamato.as_str(), vanilla.as_str());
    assert!(
        in_graveyard(&s, P2, VANILLA),
        "an 8/2 Rush Unit's attack killed the 4/4"
    );
    s.expect_in_zone(yamato.as_str(), "field");
}

#[test]
fn b16_radiant_yamato_cannon_may_not_attack_the_hero_the_turn_it_is_played() {
    let (mut s, yamato) = yamato_played(true);
    s.expect_refused(|s| s.attack(yamato.as_str(), "hero"));
    s.expect_health(P2, 30);
}

#[test]
fn b16_base_yamato_cannon_has_no_rush_and_may_not_attack_the_turn_it_is_played() {
    let (mut s, yamato) = yamato_played(false);
    assert!(!kinds(&s, &yamato).contains(&KeywordKind::Rush));
    let vanilla = s.unit_at(P2, 1).id;
    s.expect_refused(|s| s.attack(yamato.as_str(), vanilla.as_str()));
    s.expect_in_zone(vanilla.as_str(), "field");
}

// ---------------------------------------------------------------------------------------------------
// B17–B20: the weapons
// ---------------------------------------------------------------------------------------------------

#[test]
fn b17_no_aimed_weapon_lists_the_enemy_hero_as_a_target_of_its_activate() {
    for (def, name, _, _) in AIMED_WEAPONS {
        for radiant in [false, true] {
            let (s, weapon) = armed(def, radiant, json!([MENACE]));
            let menace = s.unit_at(P2, 1).id;
            let listed = activations(&s, &weapon);
            assert!(
                listed
                    .iter()
                    .any(|action| targets_of(action) == vec![the_card(&menace)]),
                "{name} (radiant {radiant}) may aim at p2's Menace: {listed:?}"
            );
            assert!(
                !listed.iter().any(aims_at_a_hero),
                "{name} (radiant {radiant}) offers a hero: {listed:?}"
            );
        }
    }
}

#[test]
fn b17_an_aimed_weapon_s_activate_at_the_enemy_hero_is_refused() {
    for (def, name, _, _) in AIMED_WEAPONS {
        for radiant in [false, true] {
            let (mut s, weapon) = armed(def, radiant, json!([MENACE]));
            let ability = ability_of(&s, &weapon);
            let at_hero = aimed(&weapon, &ability, Selection::Hero { player: P2 });
            s.expect_refused(|s| s.act(at_hero, P1, "the weapon aimed at p2's hero"));
            s.expect_health(P2, 30);
            assert!(
                hits(s.events()).is_empty(),
                "{name} (radiant {radiant}): the refused shot hit nothing"
            );
        }
    }
}

#[test]
fn b17_with_no_enemy_unit_an_aimed_weapon_still_never_aims_at_the_hero() {
    for (def, name, _, _) in AIMED_WEAPONS {
        for radiant in [false, true] {
            let ability = {
                let (probe, weapon) = armed(def, radiant, json!([MENACE]));
                ability_of(&probe, &weapon)
            };
            let (mut s, weapon) = armed(def, radiant, json!([]));
            assert!(
                !activations(&s, &weapon).iter().any(aims_at_a_hero),
                "{name} (radiant {radiant}) offers the hero with no Unit to aim at"
            );
            let at_hero = aimed(&weapon, &ability, Selection::Hero { player: P2 });
            s.expect_refused(|s| s.act(at_hero, P1, "the weapon aimed at p2's hero"));
            s.expect_health(P2, 30);
        }
    }
}

#[test]
fn b18_each_aimed_weapon_deals_its_damage_to_an_enemy_unit_on_either_face() {
    for (def, name, base, radiant_damage) in AIMED_WEAPONS {
        for radiant in [false, true] {
            let (mut s, weapon) = armed(def, radiant, json!([MENACE]));
            let menace = s.unit_at(P2, 1).id;
            let ability = ability_of(&s, &weapon);
            s.act(
                aimed(&weapon, &ability, the_card(&menace)),
                P1,
                "the weapon aimed at the Menace",
            );
            let expected = if radiant { radiant_damage } else { base };
            let on_menace: Vec<i32> = hits(s.last_events())
                .into_iter()
                .filter(|(target, _)| *target == menace)
                .map(|(_, amount)| amount)
                .collect();
            assert_eq!(on_menace, vec![expected], "{name} (radiant {radiant})");
        }
    }
}

#[test]
fn b18_each_aimed_weapon_reads_its_damage_through_its_declared_number() {
    for (def, name, base, radiant_damage) in AIMED_WEAPONS {
        for radiant in [false, true] {
            let (mut s, weapon) = armed(def, radiant, json!([MENACE]));
            let printed = if radiant { radiant_damage } else { base };
            let tuned = printed + 1;
            set_param(s.card_mut(weapon.as_str()), "damage", tuned);
            let menace = s.unit_at(P2, 1).id;
            let ability = ability_of(&s, &weapon);
            s.act(
                aimed(&weapon, &ability, the_card(&menace)),
                P1,
                "the tuned weapon aimed at the Menace",
            );
            let on_menace: Vec<i32> = hits(s.last_events())
                .into_iter()
                .filter(|(target, _)| *target == menace)
                .map(|(_, amount)| amount)
                .collect();
            assert_eq!(
                on_menace,
                vec![tuned],
                "{name} (radiant {radiant}): `damage` set to {tuned}"
            );
        }
    }
}

/// B19: the Repeater `def` activates `uses` times in a turn, each at a fresh Menace of p2's five, and the
/// next use is refused, on either face.
fn repeater_uses_a_turn(def: &str, uses: i32) {
    for radiant in [false, true] {
        let (mut s, repeater) = armed(def, radiant, json!(vec![MENACE; 5]));
        let ability = ability_of(&s, &repeater);
        for lane in 1..=uses {
            let menace = s.unit_at(P2, lane).id;
            s.act(
                aimed(&repeater, &ability, the_card(&menace)),
                P1,
                "a use of the Repeater",
            );
            assert!(
                hits(s.last_events()).iter().any(|(target, _)| *target == menace),
                "{def} (radiant {radiant}): use {lane} hit its Menace"
            );
        }
        assert!(
            activations(&s, &repeater).is_empty(),
            "{def} (radiant {radiant}): no use is left after {uses}"
        );
        let spare = s.unit_at(P2, uses + 1).id;
        let past = aimed(&repeater, &ability, the_card(&spare));
        s.expect_refused(|s| s.act(past, P1, "a use past the Repeater's count"));
        s.expect_stats(spare.as_str(), json!({ "health": 9 }));
    }
}

#[test]
fn b19_repeater_v5_activates_4_times_a_turn_and_the_5th_is_refused() {
    repeater_uses_a_turn("stinkyscape-059", 4);
}

#[test]
fn b19_repeater_v4_activates_3_times_a_turn_and_the_4th_is_refused() {
    repeater_uses_a_turn("stinkyscape-055", 3);
}

#[test]
fn b19_repeater_v3_activates_3_times_a_turn_and_the_4th_is_refused() {
    repeater_uses_a_turn("stinkyscape-051", 3);
}

#[test]
fn b20_each_flak_round_hits_the_enemy_units_and_no_hero_v3_once_v4_twice_v5_three_times() {
    for (def, rounds) in FLAKS {
        for radiant in [false, true] {
            for i in 0..10 {
                let shot = volley(def, radiant, &format!("b20-rounds-{i}"), &untuned);
                assert_eq!(
                    shot.hits.len(),
                    ENEMIES_A_ROUND * rounds,
                    "{def} (radiant {radiant}), seed {i}: {:?}",
                    shot.hits
                );
                for round in shot.hits.chunks(ENEMIES_A_ROUND) {
                    assert!(
                        round.iter().all(|(target, _)| target != ENEMY_HERO),
                        "{def}: no hero hit since #2657, {:?}",
                        shot.hits
                    );
                    let mut units = vec![round[0].0.clone(), round[1].0.clone()];
                    units.sort();
                    assert_eq!(
                        units, shot.units,
                        "{def}: each enemy Unit once a round, {:?}",
                        shot.hits
                    );
                }
            }
        }
    }
}

#[test]
fn b20_flak_never_deals_outside_least_to_most_nor_hits_its_own_side_over_many_seeds() {
    for (def, _) in FLAKS {
        for (radiant, least, most) in [(false, 1, 3), (true, 2, 4)] {
            for i in 0..20 {
                let shot = volley(def, radiant, &format!("b20-bounds-{i}"), &untuned);
                assert!(!shot.hits.is_empty(), "{def} (radiant {radiant}), seed {i}");
                for (target, amount) in &shot.hits {
                    assert!(
                        (least..=most).contains(amount),
                        "{def} (radiant {radiant}), seed {i}: {amount} is outside {least}..={most}"
                    );
                    assert!(
                        holds(&shot.units, target),
                        "{def} (radiant {radiant}), seed {i}: {target} is no enemy Unit"
                    );
                }
            }
        }
    }
}

#[test]
fn b20_base_flak_weights_its_amounts_60_30_10_lowest_first() {
    let rolled = amounts(FLAK_V5, false, "b20-weights", 150, &untuned);
    assert_eq!(rolled.len(), 150 * ENEMIES_A_ROUND * 3);
    let (ones, twos, threes) = (tally(&rolled, 1), tally(&rolled, 2), tally(&rolled, 3));
    assert_eq!(ones + twos + threes, rolled.len(), "base Flak rolls 1 to 3 only");
    assert!(
        ones > twos + threes,
        "1 is 60%: {ones} 1s, {twos} 2s, {threes} 3s"
    );
    assert!(twos > 2 * threes, "2 is 30%, 3 is 10%: {twos} 2s, {threes} 3s");
    assert!(threes > 0, "3 is 10%, never left out");
}

#[test]
fn b20_radiant_flak_s_lucky_1_rolls_twice_per_target_and_keeps_the_higher() {
    // Over 2–4 at 60/30/10, the higher of two rolls is 2 at 36%, 3 at 45% and 4 at 19%; one roll alone
    // would make 2 the most common and 4 a tenth.
    let rolled = amounts(FLAK_V5, true, "b20-lucky", 200, &untuned);
    assert_eq!(rolled.len(), 200 * ENEMIES_A_ROUND * 3);
    let (twos, threes, fours) = (tally(&rolled, 2), tally(&rolled, 3), tally(&rolled, 4));
    assert_eq!(
        twos + threes + fours,
        rolled.len(),
        "Radiant Flak rolls 2 to 4 only"
    );
    assert!(
        threes > twos,
        "the higher kept makes 3 the most common: {twos} 2s, {threes} 3s, {fours} 4s"
    );
    assert!(twos > fours, "{twos} 2s, {fours} 4s");
    assert!(
        fours * 100 > rolled.len() * 14,
        "4 comes near 19%, not 10%: {fours} of {}",
        rolled.len()
    );
}

#[test]
fn b20_each_target_and_each_round_of_a_flak_rolls_its_own_amount() {
    let mut targets_differ = false;
    let mut rounds_differ = false;
    for i in 0..40 {
        let shot = volley(FLAK_V5, false, &format!("b20-own-{i}"), &untuned);
        assert_eq!(shot.hits.len(), ENEMIES_A_ROUND * 3, "seed {i}");
        for round in shot.hits.chunks(ENEMIES_A_ROUND) {
            if round.iter().any(|(_, amount)| *amount != round[0].1) {
                targets_differ = true;
            }
        }
        let first = shot.units[0].clone();
        let on_first: Vec<i32> = shot
            .hits
            .iter()
            .filter(|(target, _)| *target == first)
            .map(|(_, amount)| *amount)
            .collect();
        if on_first.iter().any(|amount| *amount != on_first[0]) {
            rounds_differ = true;
        }
    }
    assert!(
        targets_differ,
        "in 40 volleys some round dealt its enemies different amounts"
    );
    assert!(
        rounds_differ,
        "in 40 volleys some round dealt a Unit another amount than the first"
    );
}

#[test]
fn b20_a_flak_whose_least_equals_its_most_always_deals_that_amount() {
    let two_to_two = |card: &mut CardInstance| {
        set_param(card, "least", 2);
        set_param(card, "most", 2);
    };
    for (def, rounds) in FLAKS {
        for radiant in [false, true] {
            for i in 0..5 {
                let shot = volley(def, radiant, &format!("b20-fixed-{i}"), &two_to_two);
                assert_eq!(shot.hits.len(), ENEMIES_A_ROUND * rounds, "{def}, seed {i}");
                assert!(
                    shot.hits.iter().all(|(_, amount)| *amount == 2),
                    "{def} (radiant {radiant}), seed {i}: {:?}",
                    shot.hits
                );
            }
        }
    }
}

#[test]
fn b20_a_flak_tuned_past_three_values_weights_each_past_the_third_as_the_third_and_never_passes_most() {
    // 1 to 5 over 60/30/10/10/10: 1 is half, 2 a quarter, 3, 4 and 5 a twelfth each.
    let rolled = amounts(FLAK_V5, false, "b20-wide", 100, &|card: &mut CardInstance| {
        set_param(card, "most", 5)
    });
    assert!(
        rolled.iter().all(|amount| (1..=5).contains(amount)),
        "never outside 1..=5"
    );
    let counts: Vec<usize> = (1..=5).map(|value| tally(&rolled, value)).collect();
    assert!(counts[0] > counts[1], "{counts:?}");
    assert!(
        counts[2..].iter().all(|count| counts[1] > *count && *count > 0),
        "3, 4 and 5 each weighted as the third: {counts:?}"
    );
}

#[test]
fn b20_a_flak_whose_least_and_most_a_nerf_crossed_rolls_between_them_lowest_first() {
    let crossed = |card: &mut CardInstance| {
        set_param(card, "least", 3);
        set_param(card, "most", 1);
    };
    let rolled = amounts(FLAK_V5, false, "b20-crossed", 100, &crossed);
    assert!(!rolled.is_empty());
    assert!(
        rolled.iter().all(|amount| (1..=3).contains(amount)),
        "never outside 1..=3"
    );
    let (ones, twos, threes) = (tally(&rolled, 1), tally(&rolled, 2), tally(&rolled, 3));
    assert!(
        ones > twos && twos > threes && threes > 0,
        "{ones} 1s, {twos} 2s, {threes} 3s"
    );
}
