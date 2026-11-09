//! #3483: the Stinkyscape families' shared card-test bodies.
//!
//! Beam, Repeater, Cannon and Flak at three power levels, five Miners, four Asteroids and most of
//! the Drone Cores repeat the same test bodies with different numbers. Each family file keeps thin
//! `#[test]` wrappers under its own names (CLAUDE.md rule 6; every `r<n>` name stays in the file
//! its ruling's `proven_in` lists), and the bodies live here once, taking the card id, the seed
//! tag and the numbers as arguments. No helper name holds an `r<n>` token, so `spec check` keeps
//! reading the wrappers as the proving tests. The `its_health_is_doubled` tests are gone: they only
//! reread printed Radiant stats, which a card test must not do (cards README §5, #1125).

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::prelude::*;
use jackioh_engine::testkit::*;

use crate::{
    card_def, filler, give_lucky, hits, js, next_turn_of, preview_seen, resources_in_hand, scenario,
    under_kept, under_kept_as, upgrade_number, weapon_board,
};

/// (3) 9/9 Taunt, the wall every weapon family shoots through.
pub(crate) const MENACE: &str = "core-019";
/// (1) 4/4.
pub(crate) const VANILLA: &str = "core-008";
/// (4) Spell: "Bounce all Units."
pub(crate) const FLOOD: &str = "core-017";
/// A Common Resource, for the Miners' full-hand burn.
pub(crate) const REKNITE: &str = "stinkyscape-t-reknite";
/// (2) 7/1 First Strike, destroyed by one Flak round.
pub(crate) const POINTMASTER: &str = "core-020";
/// Filler copies of Core #5 (Stockpile), as a card id.
pub(crate) const FILLER: &str = "core-005";
/// 9/9, the body the Asteroids' victim leaves and their combat partner.
pub(crate) const BODY: &str = "core-019";
/// 10/10, Indestructible, the Asteroids' combat partner.
pub(crate) const ROCK: &str = "core-066";
/// (4) Spell: destroy all permanents.
pub(crate) const NETHER: &str = "core-088";

/// The Beam's Activate at `target`.
pub(crate) fn beam_at(target: &str) -> Value {
    json!({ "ability": "beam", "targets": pick::id(target) })
}

/// The Beam's Activate at the enemy hero: refused, since #2521.
pub(crate) fn beam_at_hero() -> Value {
    json!({ "ability": "beam", "targets": [{ "pick": "hero", "player": "p2" }] })
}

/// `id` on p1's field, `radiant` or not, over `p1`/`p2`.
pub(crate) fn beam_on(id: &str, seed: &str, radiant: bool, p1: Value, p2: Value) -> Scenario {
    let mut p1 = p1;
    p1["field"] = json!([{ "def": id, "radiant": radiant }]);
    weapon_board(seed, p1, p2)
}

/// The targets `legal_actions` offers p1 for `id`'s Beam Activate, one per action.
pub(crate) fn beam_offered(s: &Scenario, id: &str) -> Vec<Value> {
    let beam = s.card(id).id.clone();
    legal_actions(s.state(), PlayerId::P1)
        .iter()
        .map(js)
        .filter(|action| action["type"] == "activate" && action["instanceId"] == beam.as_str())
        .map(|action| action["targets"].clone())
        .collect()
}

/// Three Menaces and a Vanilla, the middle one Armored: every Beam's Cleave wall.
pub(crate) fn beam_wall(id: &str, seed: &str, radiant: bool) -> (Scenario, [String; 4]) {
    let mut s = beam_on(
        id,
        seed,
        radiant,
        json!({}),
        json!({ "field": [MENACE, MENACE, MENACE, VANILLA] }),
    );
    let lanes = [1, 2, 3, 4].map(|lane| s.unit_at(PlayerId::P2, lane).id);
    s.card_mut(lanes[1].as_str())
        .granted_keywords
        .push(Keyword::Armor { n: 3 });
    (s, lanes)
}

/// R1862000: the chosen Unit's hit, then the same hit on each Unit beside it, read before the
/// first; the Armored Menace's hit is whole, since Pierce is `ignoreArmor` (R346).
pub(crate) fn beam_cleaves(id: &str, seed: &str, radiant: bool, amount: i32) {
    let (mut s, lanes) = beam_wall(id, seed, radiant);
    s.activate(id, beam_at(&lanes[1]));
    let expected: Vec<(String, i32)> = [&lanes[1], &lanes[0], &lanes[2]]
        .iter()
        .map(|id| ((*id).clone(), amount))
        .collect();
    // The Armored Menace's hit is whole: Armor would have taken 3 of it.
    assert_eq!(hits(s.last_events()), expected, "radiant {radiant}");
    s.expect_stats(lanes[3].as_str(), json!({ "health": 4 }));
    s.expect_health(PlayerId::P2, HERO_HEALTH);
}

/// "an enemy Unit": the enemy Unit is the one target offered, the enemy hero never is, and aiming
/// at the hero is refused, leaving it untouched.
pub(crate) fn beam_never_hero(id: &str, seed: &str, radiant: bool) {
    let mut s = beam_on(id, seed, radiant, json!({}), json!({ "field": [VANILLA] }));
    let unit = s.unit_at(PlayerId::P2, 1).id;
    assert_eq!(beam_offered(&s, id), vec![pick::id(&unit)], "radiant {radiant}");
    s.expect_refused(|s| s.activate(id, beam_at_hero()));
    s.expect_health(PlayerId::P2, HERO_HEALTH);
}

/// "an enemy Unit": one of yours is refused.
pub(crate) fn beam_own_unit_refused(id: &str, seed: &str) {
    let mut s = weapon_board(
        &format!("{seed}-own"),
        json!({ "field": [id, VANILLA] }),
        json!({}),
    );
    let mine = s.unit_at(PlayerId::P1, 2).id;
    s.expect_refused(|s| s.activate(id, beam_at(&mine)));
}

/// R1841000: once a turn, then cooling down, then back.
pub(crate) fn beam_cooldown_cycle(id: &str, seed: &str, damage: i32) {
    // Menaces in lanes 1 and 3: neither is beside the other, so each shot hits its own alone.
    let row = json!({ "field": [{ "def": MENACE, "lane": 1 }, { "def": MENACE, "lane": 3 }] });
    let mut s = beam_on(id, seed, false, json!({}), row);
    let [first, second] = [1, 3].map(|lane| s.unit_at(PlayerId::P2, lane).id);
    s.activate(id, beam_at(&first));
    s.expect_refused_with(|s| s.activate(id, beam_at(&second)), "already been used");
    next_turn_of(&mut s, PlayerId::P1);
    s.expect_refused_with(|s| s.activate(id, beam_at(&second)), "cooling down");
    next_turn_of(&mut s, PlayerId::P1);
    s.activate(id, beam_at(&second));
    assert_eq!(hits(s.last_events()), vec![(second, damage)]);
}

/// R1841000, R78: a Bounce clears the cooldown.
pub(crate) fn beam_bounce_clears(id: &str, seed: &str, damage: i32) {
    let mut s = beam_on(
        id,
        seed,
        false,
        json!({ "hand": [FLOOD], "mana": 4 }),
        json!({ "hand": [VANILLA], "field": [MENACE] }),
    );
    let first = s.unit_at(PlayerId::P2, 1).id;
    s.activate(id, beam_at(&first));
    s.play(FLOOD, json!({}));
    s.expect_in_zone(id, "hand");
    // p2 puts a Unit back on the field for the replayed Beam to aim at.
    s.end_turn();
    s.play(VANILLA, json!({ "zone": 1 }));
    s.end_turn();
    s.play(id, json!({ "zone": 1 }));
    let target = s.unit_at(PlayerId::P2, 1).id;
    s.activate(id, beam_at(&target));
    assert_eq!(hits(s.last_events()), vec![(target, damage)]);
}

/// R1086, R1852001: an equipped host takes the Activate and is no starship piece.
pub(crate) fn beam_equip_takes_activate(id: &str, seed: &str, damage: i32) {
    let mut s = weapon_board(
        &format!("{seed}-equip"),
        json!({ "hand": [id], "field": [VANILLA] }),
        json!({ "field": [VANILLA] }),
    );
    let host = s.unit_at(PlayerId::P1, 1).id;
    let target = s.unit_at(PlayerId::P2, 1).id;
    s.play(id, json!({ "zone": 1, "equip": true }));
    let fused = s.card(host.as_str()).clone();
    assert!(!unit_has(s.state(), &fused, KeywordKind::StarshipPiece));
    s.activate(host.as_str(), beam_at(&target));
    assert_eq!(hits(s.last_events()), vec![(target, damage)]);
}

/// The number is the declared `damage` (R386), read through `param`.
pub(crate) fn beam_param_reads(id: &str, seed: &str) {
    let mut s = beam_on(id, seed, false, json!({}), json!({ "field": [VANILLA] }));
    let amount = upgrade_number(&mut s, id, "damage");
    let target = s.unit_at(PlayerId::P2, 1).id;
    s.activate(id, beam_at(&target));
    assert_eq!(hits(s.last_events()), vec![(target, amount)]);
}

/// R767000: the script kept for the replays of v0.5.11 and before may aim at the enemy hero,
/// whose hit is halved, rounded down, with no Cleave off it (R1841001).
pub(crate) fn beam_kept_halves_hero(id: &str, seed: &str, damage: i32, kept: fn() -> CardScripts) {
    under_kept(id, kept(), || {
        let halved = damage / HALF_DAMAGE_TO_HEROES_DIVISOR;
        let mut s = beam_on(id, seed, false, json!({}), json!({ "field": [MENACE, VANILLA] }));
        s.activate(id, beam_at_hero());
        assert_eq!(hits(s.last_events()), vec![("hero-p2".to_string(), halved)]);
        s.expect_health(PlayerId::P2, HERO_HEALTH - halved);
    });
}

/// The Repeater's Activate at `target`.
pub(crate) fn rep_at(target: &str) -> Value {
    json!({ "ability": "repeater", "targets": pick::id(target) })
}

/// The Repeater's Activate at the enemy hero: refused, since #2521.
pub(crate) fn rep_at_hero() -> Value {
    json!({ "ability": "repeater", "targets": [{ "pick": "hero", "player": "p2" }] })
}

/// `id` on p1's field, `radiant` or not, over p2's `p2`.
pub(crate) fn rep_on(id: &str, seed: &str, radiant: bool, p2: Value) -> Scenario {
    weapon_board(seed, json!({ "field": [{ "def": id, "radiant": radiant }] }), p2)
}

/// The targets `legal_actions` offers p1 for `id`'s Repeater Activate, one per action.
pub(crate) fn rep_offered(s: &Scenario, id: &str) -> Vec<Value> {
    let repeater = s.card(id).id.clone();
    legal_actions(s.state(), PlayerId::P1)
        .iter()
        .map(js)
        .filter(|action| action["type"] == "activate" && action["instanceId"] == repeater.as_str())
        .map(|action| action["targets"].clone())
        .collect()
}

/// R1862000: one hit on an Armored Menace between two others, whole, and none beside it: Pierce
/// is `ignoreArmor` (R346).
pub(crate) fn rep_pierces(id: &str, seed: &str, radiant: bool, amount: i32) {
    let mut s = rep_on(id, seed, radiant, json!({ "field": [MENACE, MENACE, MENACE] }));
    let target = s.unit_at(PlayerId::P2, 2).id;
    s.card_mut(target.as_str())
        .granted_keywords
        .push(Keyword::Armor { n: 3 });
    s.activate(id, rep_at(&target));
    assert_eq!(hits(s.last_events()), vec![(target, amount)], "radiant {radiant}");
    s.expect_health(PlayerId::P2, HERO_HEALTH);
}

/// "an enemy Unit": the enemy Unit is the one target offered, the enemy hero never is, and aiming
/// at the hero is refused, leaving it untouched.
pub(crate) fn rep_never_hero(id: &str, seed: &str, radiant: bool) {
    let mut s = rep_on(id, seed, radiant, json!({ "field": [VANILLA] }));
    let unit = s.unit_at(PlayerId::P2, 1).id;
    assert_eq!(rep_offered(&s, id), vec![pick::id(&unit)], "radiant {radiant}");
    s.expect_refused(|s| s.activate(id, rep_at_hero()));
    s.expect_health(PlayerId::P2, HERO_HEALTH);
}

/// R1857003: `uses` uses a turn, one per Vanilla in lanes 1 to `uses`, the last of them landing;
/// the next, at the Vanilla in lane `uses` + 1, is refused; the turn after, it is back (no
/// cooldown).
pub(crate) fn rep_uses_a_turn(id: &str, seed: &str, radiant: bool, uses: i32, amount: i32) {
    let row = json!({ "field": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA] });
    let mut s = rep_on(id, seed, radiant, row);
    for lane in 1..uses {
        let target = s.unit_at(PlayerId::P2, lane).id;
        s.activate(id, rep_at(&target));
    }
    let last = s.unit_at(PlayerId::P2, uses).id;
    s.activate(id, rep_at(&last));
    assert_eq!(hits(s.last_events()), vec![(last, amount)], "use {uses} lands");
    let next = s.unit_at(PlayerId::P2, uses + 1).id;
    s.expect_refused_with(
        |s| s.activate(id, rep_at(&next)),
        &format!("that ability has been used {uses} times this turn"),
    );
    next_turn_of(&mut s, PlayerId::P1);
    s.activate(id, rep_at(&next));
    assert_eq!(hits(s.last_events()), vec![(next, amount)], "radiant {radiant}");
}

/// R1086, R1852001: an equipped host takes the Activate and is no starship piece.
pub(crate) fn rep_equip_takes_activate(id: &str, seed: &str, damage: i32) {
    let mut s = weapon_board(
        &format!("{seed}-equip"),
        json!({ "hand": [id], "field": [VANILLA] }),
        json!({ "field": [VANILLA] }),
    );
    let host = s.unit_at(PlayerId::P1, 1).id;
    let target = s.unit_at(PlayerId::P2, 1).id;
    s.play(id, json!({ "zone": 1, "equip": true }));
    let fused = s.card(host.as_str()).clone();
    assert!(!unit_has(s.state(), &fused, KeywordKind::StarshipPiece));
    s.activate(host.as_str(), rep_at(&target));
    assert_eq!(hits(s.last_events()), vec![(target, damage)]);
}

/// The number is the declared `damage` (R386), read through `param`.
pub(crate) fn rep_param_reads(id: &str, seed: &str) {
    let mut s = rep_on(id, seed, false, json!({ "field": [VANILLA] }));
    let amount = upgrade_number(&mut s, id, "damage");
    let target = s.unit_at(PlayerId::P2, 1).id;
    s.activate(id, rep_at(&target));
    assert_eq!(hits(s.last_events()), vec![(target, amount)]);
}

/// R767000: the script kept for the replays of v0.5.11 and before may aim at the enemy hero,
/// whose hit is halved, rounded down (R1841001), `old_uses` times a turn; the next is refused.
pub(crate) fn rep_kept_halves_hero(
    id: &str,
    seed: &str,
    radiant_damage: i32,
    old_uses: i32,
    kept: fn() -> CardScripts,
) {
    under_kept(id, kept(), || {
        let halved = radiant_damage / HALF_DAMAGE_TO_HEROES_DIVISOR;
        let mut s = rep_on(id, seed, true, json!({ "field": [VANILLA] }));
        for _ in 0..old_uses {
            s.activate(id, rep_at_hero());
            assert_eq!(hits(s.last_events()), vec![("hero-p2".to_string(), halved)]);
        }
        s.expect_health(PlayerId::P2, HERO_HEALTH - old_uses * halved);
        s.expect_refused(|s| s.activate(id, rep_at_hero()));
    });
}

/// The Cannon's Activate at `target`.
pub(crate) fn can_at(target: &str) -> Value {
    json!({ "ability": "cannon", "targets": pick::id(target) })
}

/// The Cannon's Activate at the enemy hero: refused, since #2521.
pub(crate) fn can_at_hero() -> Value {
    json!({ "ability": "cannon", "targets": [{ "pick": "hero", "player": "p2" }] })
}

/// `id` on p1's field, `radiant` or not, over p2's `p2`.
pub(crate) fn can_on(id: &str, seed: &str, radiant: bool, p2: Value) -> Scenario {
    weapon_board(seed, json!({ "field": [{ "def": id, "radiant": radiant }] }), p2)
}

/// The targets `legal_actions` offers p1 for `id`'s Cannon Activate, one per action.
pub(crate) fn can_offered(s: &Scenario, id: &str) -> Vec<Value> {
    let cannon = s.card(id).id.clone();
    legal_actions(s.state(), PlayerId::P1)
        .iter()
        .map(js)
        .filter(|action| action["type"] == "activate" && action["instanceId"] == cannon.as_str())
        .map(|action| action["targets"].clone())
        .collect()
}

/// The Armor the Cannon's target wears in its test.
pub(crate) const CANNON_ARMOR: i32 = 3;

/// R1862000: one hit on an Armored Menace between two others, the Armor taking its part, and none
/// beside it.
pub(crate) fn can_hits(id: &str, seed: &str, radiant: bool, amount: i32) {
    let mut s = can_on(id, seed, radiant, json!({ "field": [MENACE, MENACE, MENACE] }));
    let target = s.unit_at(PlayerId::P2, 2).id;
    s.card_mut(target.as_str())
        .granted_keywords
        .push(Keyword::Armor { n: CANNON_ARMOR });
    s.activate(id, can_at(&target));
    assert_eq!(
        hits(s.last_events()),
        vec![(target, amount - CANNON_ARMOR)],
        "radiant {radiant}"
    );
    s.expect_health(PlayerId::P2, HERO_HEALTH);
}

/// "an enemy Unit": the enemy Unit is the one target offered, the enemy hero never is, and aiming
/// at the hero is refused, leaving it untouched.
pub(crate) fn can_never_hero(id: &str, seed: &str, radiant: bool) {
    let mut s = can_on(id, seed, radiant, json!({ "field": [VANILLA] }));
    let unit = s.unit_at(PlayerId::P2, 1).id;
    assert_eq!(can_offered(&s, id), vec![pick::id(&unit)], "radiant {radiant}");
    s.expect_refused(|s| s.activate(id, can_at_hero()));
    s.expect_health(PlayerId::P2, HERO_HEALTH);
}

/// R384: once a turn; a second use refused.
pub(crate) fn can_once_a_turn(id: &str, seed: &str, damage: i32) {
    let mut s = can_on(id, seed, false, json!({ "field": [MENACE, MENACE] }));
    let [first, second] = [1, 2].map(|lane| s.unit_at(PlayerId::P2, lane).id);
    s.activate(id, can_at(&first));
    s.expect_refused_with(|s| s.activate(id, can_at(&second)), "already been used");
    next_turn_of(&mut s, PlayerId::P1);
    s.activate(id, can_at(&second));
    assert_eq!(hits(s.last_events()), vec![(second, damage)]);
}

/// R1086, R1852001: an equipped host takes the Activate and is no starship piece.
pub(crate) fn can_equip_takes_activate(id: &str, seed: &str, damage: i32) {
    let mut s = weapon_board(
        &format!("{seed}-equip"),
        json!({ "hand": [id], "field": [VANILLA] }),
        json!({ "field": [VANILLA] }),
    );
    let host = s.unit_at(PlayerId::P1, 1).id;
    let target = s.unit_at(PlayerId::P2, 1).id;
    s.play(id, json!({ "zone": 1, "equip": true }));
    let fused = s.card(host.as_str()).clone();
    assert!(!unit_has(s.state(), &fused, KeywordKind::StarshipPiece));
    s.activate(host.as_str(), can_at(&target));
    assert_eq!(hits(s.last_events()), vec![(target, damage)]);
}

/// The number is the declared `damage` (R386), read through `param`.
pub(crate) fn can_param_reads(id: &str, seed: &str) {
    let mut s = can_on(id, seed, false, json!({ "field": [MENACE] }));
    let amount = upgrade_number(&mut s, id, "damage");
    let target = s.unit_at(PlayerId::P2, 1).id;
    s.activate(id, can_at(&target));
    assert_eq!(hits(s.last_events()), vec![(target, amount)]);
}

/// R767000: the script kept for the replays of v0.5.11 and before may aim at the enemy hero,
/// whose hit is halved, rounded down (R1841001).
pub(crate) fn can_kept_halves_hero(id: &str, seed: &str, damage: i32, kept: fn() -> CardScripts) {
    under_kept(id, kept(), || {
        let halved = damage / HALF_DAMAGE_TO_HEROES_DIVISOR;
        let mut s = can_on(id, seed, false, json!({ "field": [VANILLA] }));
        s.activate(id, can_at_hero());
        assert_eq!(hits(s.last_events()), vec![("hero-p2".to_string(), halved)]);
        s.expect_health(PlayerId::P2, HERO_HEALTH - halved);
    });
}

/// The printed Flak ranges, base and Radiant, and the Radiant face's Lucky.
pub(crate) const FLAK_BASE_RANGE: (i32, i32) = (1, 3);
pub(crate) const FLAK_RADIANT_RANGE: (i32, i32) = (2, 4);
pub(crate) const FLAK_RADIANT_LUCKY: i32 = 1;
/// The printed odds as the test reads them, lowest first.
pub(crate) const FLAK_PRINTED: [i32; 3] = [60, 30, 10];

/// A Menace no activation can bring down, so every round hits it.
pub(crate) fn flak_wall() -> Value {
    json!({ "def": MENACE, "statsOverride": { "attack": 9, "health": 40 } })
}

/// `id` on p1's field beside a Vanilla, `radiant` or not, over p2's `p2`.
pub(crate) fn flak_on(id: &str, seed: &str, radiant: bool, p2: Value) -> Scenario {
    weapon_board(
        seed,
        json!({ "field": [{ "def": id, "radiant": radiant }, VANILLA] }),
        p2,
    )
}

/// Two walls for p2.
pub(crate) fn flak_walls() -> Value {
    json!({ "field": [flak_wall(), flak_wall()] })
}

/// One roll over `lo..=hi` as the text reads: 60, 30 and 10 from the low end, a value past the
/// third weighted as the third.
pub(crate) fn flak_printed_roll(rng: &mut Rng, (lo, hi): (i32, i32)) -> i32 {
    let weight = |amount: i32| FLAK_PRINTED[((amount - lo) as usize).min(FLAK_PRINTED.len() - 1)];
    let mut at = rng.int((lo..=hi).map(weight).sum());
    for amount in lo..=hi {
        if at < weight(amount) {
            return amount;
        }
        at -= weight(amount);
    }
    hi
}

/// Activates `card` against enemies that all live through it and holds its hits to the rolls a
/// copy of the match rng gives: each enemy Unit in lane order, round after round, each its own
/// roll over `range` with `lucky` more kept at the higher, and no other draw.
pub(crate) fn flak_fire(
    s: &mut Scenario,
    card: &str,
    range: (i32, i32),
    lucky: i32,
    rounds: i32,
) -> Vec<(String, i32)> {
    let targets: Vec<String> = (1..=UNIT_ZONES)
        .filter_map(|lane| s.unit(PlayerId::P2, lane))
        .map(|unit| unit.id)
        .collect();
    let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
    let health = s.state().players.p2.hero.health;
    s.activate(card, json!({}));
    let mut expected: Vec<(String, i32)> = Vec::new();
    for _ in 0..rounds {
        for target in &targets {
            let amount = rng.lucky(lucky, |rng| flak_printed_roll(rng, range), Ord::max);
            expected.push((target.clone(), amount));
        }
    }
    let got = hits(s.last_events());
    assert_eq!(got, expected, "one roll per target and round");
    assert_eq!(
        s.state().rng_cursor,
        rng.cursor(),
        "1 + Lucky draws per hit, no other"
    );
    assert!(
        got.iter().all(|(target, _)| target != "hero-p2"),
        "no hero hit since #2657"
    );
    s.expect_health(PlayerId::P2, health);
    s.expect_health(PlayerId::P1, HERO_HEALTH);
    got
}

/// What activations over `seeds` seeds of the given face against two walls showed: how often each
/// amount came up, whether the two walls ever took different amounts in one round, and whether a
/// wall's hit in a later round ever differed from its first.
pub(crate) struct FlakSeen {
    pub(crate) counts: IndexMap<i32, usize>,
    pub(crate) differed: bool,
    pub(crate) rerolled: bool,
}

pub(crate) fn flak_over_the_seeds(
    id: &str,
    prefix: &str,
    radiant: bool,
    range: (i32, i32),
    lucky: i32,
    rounds: i32,
    seeds: usize,
) -> FlakSeen {
    let mut seen = FlakSeen {
        counts: IndexMap::new(),
        differed: false,
        rerolled: false,
    };
    for seed in 0..seeds {
        let mut s = flak_on(
            id,
            &format!("{prefix}-seeds-{radiant}-{seed}"),
            radiant,
            flak_walls(),
        );
        let got = flak_fire(&mut s, id, range, lucky, rounds);
        // Two walls a round.
        let per_round = got.len() / rounds as usize;
        seen.differed |= got[0].1 != got[1].1;
        seen.rerolled |= (1..rounds as usize).any(|round| got[0].1 != got[round * per_round].1);
        for (_, amount) in got {
            *seen.counts.entry(amount).or_default() += 1;
        }
    }
    seen
}

/// R767000: the script kept for the replays of v0.5.9f and before rolls for the enemy hero too,
/// after each enemy Unit.
pub(crate) fn flak_kept_v9(id: &str, seed: &str, rounds: i32, kept: fn() -> CardScripts) {
    under_kept(id, kept(), || {
        let mut s = flak_on(id, seed, false, json!({ "field": [flak_wall()] }));
        let target = s.unit_at(PlayerId::P2, 1).id;
        s.activate(id, json!({}));
        let got = hits(s.last_events());
        let round: Vec<String> = vec![target, "hero-p2".to_string()];
        let expected: Vec<String> = (0..rounds).flat_map(|_| round.clone()).collect();
        let targets: Vec<String> = got.into_iter().map(|(target, _)| target).collect();
        assert_eq!(targets, expected);
    });
}

/// R767000: the script kept for the replays of v0.5.11 and before dealt its declared `damage` to
/// each enemy Unit and then the enemy hero, round after round.
pub(crate) fn flak_kept_v11(id: &str, seed: &str, rounds: i32, kept: fn() -> CardScripts) {
    // v0.5.11's number, which its definition declared and the replay carries.
    const DAMAGE: i32 = 2;
    let old = |def: &mut CardDef| {
        let declared: Vec<Param> = serde_json::from_value(json!([{
            "key": "damage", "base": DAMAGE, "radiant": 4, "better": "up", "step": 1, "min": 1
        }]))
        .expect("v0.5.11's damage");
        def.params.get_or_insert_with(Vec::new).extend(declared);
    };
    under_kept_as(id, kept(), old, || {
        let mut s = flak_on(id, seed, false, json!({ "field": [flak_wall()] }));
        let target = s.unit_at(PlayerId::P2, 1).id;
        let cursor = s.state().rng_cursor;
        s.activate(id, json!({}));
        let round = [(target, DAMAGE), ("hero-p2".to_string(), DAMAGE)];
        let expected: Vec<(String, i32)> = (0..rounds).flat_map(|_| round.clone()).collect();
        assert_eq!(hits(s.last_events()), expected);
        assert_eq!(s.state().rng_cursor, cursor, "no roll drawn");
    });
}

/// R1862001: a Unit one round destroys is not hit by the next.
pub(crate) fn flak_kill_skips_destroyed(id: &str, prefix: &str) {
    let mut s = flak_on(
        id,
        &format!("{prefix}-kill"),
        false,
        json!({ "field": [POINTMASTER, flak_wall()] }),
    );
    let pointmaster = s.unit_at(PlayerId::P2, 1).id;
    s.activate(id, json!({}));
    let got = hits(s.last_events());
    let on_it = got.iter().filter(|(target, _)| *target == pointmaster).count();
    assert_eq!(on_it, 1);
    s.expect_in_zone(pointmaster.as_str(), "graveyard");
    assert!(
        got.iter().all(|(target, _)| target != "hero-p2"),
        "no hero hit since #2657"
    );
    s.expect_health(PlayerId::P2, HERO_HEALTH);
}

/// R2521001, R1438: with Lucky each hit rolls again, keeping the higher.
pub(crate) fn flak_lucky_again(id: &str, prefix: &str, range: (i32, i32), seeds: usize, rounds: i32) {
    for seed in 0..seeds {
        let mut s = flak_on(id, &format!("{prefix}-given-{seed}"), false, flak_walls());
        give_lucky(&mut s, id, 1);
        flak_fire(&mut s, id, range, 1, rounds);
    }
}

/// R384: once a turn; a second use refused.
pub(crate) fn flak_once_a_turn(id: &str, prefix: &str, range: (i32, i32), rounds: i32) {
    let mut s = flak_on(id, &format!("{prefix}-once"), false, json!({}));
    flak_fire(&mut s, id, range, 0, rounds);
    s.expect_refused_with(|s| s.activate(id, json!({})), "already been used");
    next_turn_of(&mut s, PlayerId::P1);
    flak_fire(&mut s, id, range, 0, rounds);
}

/// R1086, R1852001: an equipped host takes the Activate and is no starship piece.
pub(crate) fn flak_equip_takes_activate(id: &str, prefix: &str, range: (i32, i32), rounds: i32) {
    let mut s = weapon_board(
        &format!("{prefix}-equip"),
        json!({ "hand": [id], "field": [VANILLA] }),
        flak_walls(),
    );
    let host = s.unit_at(PlayerId::P1, 1).id;
    s.play(id, json!({ "zone": 1, "equip": true }));
    let fused = s.card(host.as_str()).clone();
    assert!(!unit_has(s.state(), &fused, KeywordKind::StarshipPiece));
    flak_fire(&mut s, host.as_str(), range, 0, rounds);
}

/// R386, R2521001: an Upgrade of `most` rolls past the third at the third's weight.
pub(crate) fn flak_upgrade_most(id: &str, prefix: &str, range: (i32, i32), seeds: usize, rounds: i32) {
    let mut seen: IndexSet<i32> = IndexSet::new();
    for seed in 0..seeds {
        let mut s = flak_on(id, &format!("{prefix}-most-{seed}"), false, flak_walls());
        let most = upgrade_number(&mut s, id, "most");
        seen.extend(
            flak_fire(&mut s, id, (range.0, most), 0, rounds)
                .into_iter()
                .map(|(_, amount)| amount),
        );
    }
    assert!(seen.contains(&(range.1 + 1)), "{seen:?}");
}

/// R386, R2521001: an Upgrade of `least` lifts the low end.
pub(crate) fn flak_upgrade_least(id: &str, prefix: &str, range: (i32, i32), rounds: i32) {
    let mut s = flak_on(id, &format!("{prefix}-least"), false, flak_walls());
    let least = upgrade_number(&mut s, id, "least");
    let got = flak_fire(&mut s, id, (least, range.1), 0, rounds);
    assert!(got.iter().all(|(_, amount)| *amount >= least), "{got:?}");
}

/// R2521001: a lift of `least` past `most` rolls between the two.
pub(crate) fn flak_roll_between(id: &str, prefix: &str, range: (i32, i32), seeds: usize, rounds: i32) {
    for seed in 0..seeds {
        let mut s = flak_on(id, &format!("{prefix}-crossed-{seed}"), false, flak_walls());
        step_param(s.card_mut(id), "least", range.1);
        // Least past most: between them, the low end first.
        flak_fire(&mut s, id, (range.1, range.1 + 1), 0, rounds);
    }
}

/// R2521001: a Radiant first hit is the higher of its two rolls.
pub(crate) fn flak_radiant_pair(
    id: &str,
    prefix: &str,
    range: (i32, i32),
    lucky: i32,
    seeds: usize,
    rounds: i32,
) {
    for seed in 0..seeds {
        let mut s = flak_on(id, &format!("{prefix}-radiant-pair-{seed}"), true, flak_walls());
        let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
        let first = flak_printed_roll(&mut rng, range);
        let second = flak_printed_roll(&mut rng, range);
        let got = flak_fire(&mut s, id, range, lucky, rounds);
        assert_eq!(got[0].1, first.max(second));
    }
}

/// A hero with room for three Radiant activations' worth of hits.
pub(crate) const FLAK_DEEP_HEALTH: i32 = 100;

/// R1857003: the Radiant face's Activate 3 is three uses a turn; the fourth refused.
pub(crate) fn flak_uses_limit(id: &str, prefix: &str, range: (i32, i32), lucky: i32, uses: i32, rounds: i32) {
    let mut s = flak_on(
        id,
        &format!("{prefix}-uses"),
        true,
        json!({ "health": FLAK_DEEP_HEALTH }),
    );
    for _ in 0..uses {
        flak_fire(&mut s, id, range, lucky, rounds);
    }
    s.expect_refused_with(
        |s| s.activate(id, json!({})),
        &format!("that ability has been used {uses} times this turn"),
    );
}

/// `id` on p1's field, base face unless `radiant`, under `seed`.
pub(crate) fn miner_board(id: &str, seed: &str, radiant: bool) -> Scenario {
    weapon_board(
        seed,
        json!({ "field": [{ "def": id, "radiant": radiant }] }),
        json!({}),
    )
}

/// The Miner's Activate.
pub(crate) fn miner_dig() -> Value {
    json!({ "ability": "mine" })
}

/// The Resources in p1's hand.
pub(crate) fn miner_mined(s: &Scenario) -> Vec<CardInstance> {
    resources_in_hand_of(s.state(), PlayerId::P1)
}

/// R1872000, R2250000: its Activate gains one random Resource into the hand; the roll replays
/// from the seed (R1840002).
pub(crate) fn miner_gains_one(id: &str, seed: &str) {
    let mut s = miner_board(id, seed, false);
    let hand = s.hand(PlayerId::P1).len();
    s.activate(id, miner_dig());
    let in_hand = miner_mined(&s);
    assert_eq!(in_hand.len(), 1);
    assert!(
        card_def(&in_hand[0].def_id).resource.is_some(),
        "{}",
        in_hand[0].def_id
    );
    assert!(!in_hand[0].radiant);
    assert_eq!(s.hand(PlayerId::P1).len(), hand + 1, "it takes a hand slot");
    // R1840002: the roll replays from the seed.
    let mut again = miner_board(id, seed, false);
    again.activate(id, miner_dig());
    assert_eq!(
        resources_in_hand_of(again.state(), PlayerId::P1)[0].def_id,
        in_hand[0].def_id
    );
}

/// R1840001, R2250000: into a full hand the Resource burns.
pub(crate) fn miner_burns_when_full(id: &str, seed: &str) {
    let full = vec![REKNITE; HAND_CAP as usize];
    let mut s = weapon_board(
        &format!("{seed}-full"),
        json!({ "field": [id], "hand": full }),
        json!({}),
    );
    s.activate(id, miner_dig());
    assert_eq!(s.hand(PlayerId::P1).len(), HAND_CAP as usize);
    assert!(
        s.last_events()
            .iter()
            .any(|event| event.event_type() == GameEventType::Burned)
    );
}

/// R384: `uses` uses a turn, then a refusal, and again next turn. One use a turn is refused as
/// "already been used"; more as "used `uses` times this turn".
pub(crate) fn miner_uses_limit(id: &str, seed: &str, uses: i32) {
    let mut s = miner_board(id, &format!("{seed}-uses"), false);
    for _ in 0..uses {
        s.activate(id, miner_dig());
    }
    if uses == 1 {
        s.expect_refused_with(|s| s.activate(id, miner_dig()), "already been used");
    } else {
        let refusal = format!("used {uses} times this turn");
        s.expect_refused_with(|s| s.activate(id, miner_dig()), &refusal);
    }
    assert_eq!(miner_mined(&s).len(), uses as usize);
    next_turn_of(&mut s, PlayerId::P1);
    s.activate(id, miner_dig());
    assert_eq!(miner_mined(&s).len(), uses as usize + 1);
}

/// R1841000: once a turn, then cooling down, then back.
pub(crate) fn miner_cooldown(id: &str, seed: &str) {
    let mut s = miner_board(id, &format!("{seed}-cooldown"), false);
    s.activate(id, miner_dig());
    s.expect_refused_with(|s| s.activate(id, miner_dig()), "already been used");
    next_turn_of(&mut s, PlayerId::P1);
    s.expect_refused_with(|s| s.activate(id, miner_dig()), "cooling down");
    next_turn_of(&mut s, PlayerId::P1);
    s.activate(id, miner_dig());
    assert_eq!(miner_mined(&s).len(), 2);
}

/// R1086, R1872000: equipped onto a Unit, the host takes the Activate.
pub(crate) fn miner_equip(id: &str, seed: &str) {
    let mut s = weapon_board(
        &format!("{seed}-equip"),
        json!({ "hand": [id], "field": [VANILLA], "mana": 10 }),
        json!({}),
    );
    let host = s.unit_at(PlayerId::P1, 1).id;
    s.play(id, json!({ "zone": 1, "equip": true }));
    assert_eq!(s.unit_at(PlayerId::P1, 1).id, host);
    s.activate(host.as_str(), miner_dig());
    assert_eq!(miner_mined(&s).len(), 1);
}

/// The Radiant face's Resource is Radiant.
pub(crate) fn miner_radiant_gain(id: &str, seed: &str) {
    let mut s = miner_board(id, &format!("{seed}-radiant"), true);
    s.activate(id, miner_dig());
    let in_hand = miner_mined(&s);
    assert_eq!(in_hand.len(), 1);
    assert!(in_hand[0].radiant);
}

/// The Asteroid on p1's field, `radiant` or not, a Twisting Nether and a filler in hand.
pub(crate) fn asteroid_node(id: &str, radiant: bool) -> Scenario {
    scenario(json!({
        "p1": {
            "hand": [NETHER, FILLER],
            "library": filler(4),
            "field": [{ "def": id, "radiant": radiant }],
        },
        "p2": { "hand": [FILLER], "library": filler(4), "field": [BODY] },
    }))
}

/// Its Activate.
pub(crate) fn asteroid_mine(id: &str, s: &mut Scenario) {
    s.activate(id, json!({ "ability": "mine" }));
}

/// R1840006: the Activate damages, then tributes, and pays.
pub(crate) fn asteroid_activate_pays(id: &str, self_damage: i32, resources: usize) {
    let mut s = asteroid_node(id, false);
    asteroid_mine(id, &mut s);
    s.expect_health(PlayerId::P1, 30 - self_damage);
    s.expect_in_zone(id, "graveyard");
    let in_hand = resources_in_hand(&s, PlayerId::P1);
    assert_eq!(in_hand.len(), resources);
    assert!(in_hand.iter().all(|card| !card.radiant));
}

/// Any other death pays too; an exile or a Bounce nothing. Here: destroyed by Twisting Nether,
/// and in combat against an Indestructible.
pub(crate) fn asteroid_death_pays(id: &str, resources: usize) {
    let mut s = asteroid_node(id, false);
    s.play(NETHER, json!({}));
    assert_eq!(
        resources_in_hand(&s, PlayerId::P1).len(),
        resources,
        "Twisting Nether"
    );

    let mut s = scenario(json!({
        "active": "p2",
        "p1": { "hand": [FILLER], "library": filler(4), "field": [id] },
        "p2": { "hand": [FILLER], "library": filler(4), "field": [ROCK] },
    }));
    let attacker = s.unit_at(PlayerId::P2, 1);
    let node = s.unit_at(PlayerId::P1, 1);
    s.attack(&attacker, &node);
    s.expect_in_zone(&node, "graveyard");
    assert_eq!(resources_in_hand(&s, PlayerId::P1).len(), resources, "in combat");
    assert!(resources_in_hand(&s, PlayerId::P2).is_empty());
}

/// Can't attack, and Can't switch: it never attacks and never changes position.
pub(crate) fn asteroid_never_attacks(id: &str) {
    let mut s = asteroid_node(id, false);
    s.expect_refused(|s| s.attack(id, "hero"));
    s.expect_refused(|s| s.switch_position(id));
    let target = s.unit_at(PlayerId::P2, 1);
    s.expect_refused(|s| s.attack(id, &target));
}

/// `resources` and `selfDamage` read through `param` (R386).
pub(crate) fn asteroid_params(id: &str) {
    let mut s = asteroid_node(id, false);
    set_param(s.card_mut(id), "resources", 3);
    set_param(s.card_mut(id), "selfDamage", 1);
    asteroid_mine(id, &mut s);
    s.expect_health(PlayerId::P1, 29);
    assert_eq!(resources_in_hand(&s, PlayerId::P1).len(), 3);
}

/// The Radiant face's Activate is the tribute alone, and its Resources are Radiant.
pub(crate) fn asteroid_radiant(id: &str, resources: usize) {
    let mut s = asteroid_node(id, true);
    asteroid_mine(id, &mut s);
    s.expect_health(PlayerId::P1, 30);
    s.expect_in_zone(id, "graveyard");
    let in_hand = resources_in_hand(&s, PlayerId::P1);
    assert_eq!(in_hand.len(), resources);
    assert!(in_hand.iter().all(|card| card.radiant));
}

/// R2476000, R1855002: with no rider it previews nothing; with an Anomaly board's rider it names
/// the pool after "random Stinkyscape Resource" (Radiant: "random Radiant Stinkyscape Resource"),
/// to both seats, and its Death's Resources come from that pool alone.
pub(crate) fn asteroid_rider_pool(id: &str) {
    for radiant in [false, true] {
        let mut s = asteroid_node(id, radiant);
        let rock = s.unit_at(PlayerId::P1, 1).id;
        assert!(
            preview_seen(&s, PlayerId::P1, &rock).is_empty(),
            "radiant {radiant}: no rider"
        );
        let pool = ["stinkyscape-t-narcor", "stinkyscape-t-red-narcor"];
        s.card_mut(&rock).memory.insert(
            jackioh_engine::effects::RESOURCE_POOL_KEY.to_string(),
            json!(pool),
        );
        let label = if radiant {
            "random Radiant Stinkyscape Resource"
        } else {
            "random Stinkyscape Resource"
        };
        let named = vec![(label.to_string(), 2, Some("Narcor or Red Narcor".to_string()))];
        assert_eq!(preview_seen(&s, PlayerId::P1, &rock), named, "radiant {radiant}");
        assert_eq!(preview_seen(&s, PlayerId::P2, &rock), named, "radiant {radiant}");
        asteroid_mine(id, &mut s);
        let gained = resources_in_hand(&s, PlayerId::P1);
        assert!(!gained.is_empty(), "radiant {radiant}");
        assert!(
            gained.iter().all(|card| pool.contains(&card.def_id.as_str())),
            "radiant {radiant}"
        );
    }
}

/// The single-summon cores' setup (Drone Cores II and V): `core` in hand, `radiant` or not, over
/// `field`; played.
pub(crate) fn drone_single_cast(core: &str, seed: &str, radiant: bool, field: Value) -> Scenario {
    let mut s = scenario(json!({
        "seed": seed,
        "p1": { "hand": [{ "def": core, "radiant": radiant }, FILLER], "field": field, "library": filler(4) },
        "p2": { "hand": [FILLER], "library": filler(4) },
    }));
    s.play(core, json!({}));
    s
}

/// p1's unit row, each lane the def id and Radiant flag there, if any.
pub(crate) fn drone_unit_row(s: &Scenario) -> Vec<Option<(String, bool)>> {
    (1..=5)
        .map(|lane| s.unit(PlayerId::P1, lane).map(|unit| (unit.def_id, unit.radiant)))
        .collect()
}

/// p1's backrow, the same way.
pub(crate) fn drone_backrow_row(s: &Scenario) -> Vec<Option<(String, bool)>> {
    (1..=5)
        .map(|lane| {
            s.backrow(PlayerId::P1, lane)
                .map(|card| (card.def_id, card.radiant))
        })
        .collect()
}

/// A `token` summon, `radiant` or not, as the row helpers read it.
pub(crate) fn drone_cell(token: &str, radiant: bool) -> Option<(String, bool)> {
    Some((token.to_string(), radiant))
}

/// The multi-summon cores' setup (Drone Cores III, IV and VI): `core` in hand, `radiant` or not,
/// over `field`.
pub(crate) fn drone_multi_setup(core: &str, seed: &str, radiant: bool, field: Value) -> Scenario {
    scenario(json!({
        "seed": seed,
        "p1": { "hand": [{ "def": core, "radiant": radiant }, FILLER], "field": field, "library": filler(4) },
        "p2": { "hand": [FILLER], "library": filler(4) },
    }))
}

/// p1 holds one `core` in hand and one in the deck, `mana` unspent as its turn ends.
pub(crate) fn drone_floating(core: &str, mana: i32, radiant: bool) -> (Scenario, String, String) {
    let s = scenario(json!({
        "p1": {
            "hand": [{ "def": core, "radiant": radiant }, FILLER],
            "library": [{ "def": core, "radiant": radiant }, FILLER, FILLER],
            "mana": mana,
        },
        "p2": { "hand": [FILLER], "library": filler(2) },
    }));
    let hand = s
        .hand(PlayerId::P1)
        .into_iter()
        .find(|card| card.def_id == core)
        .map(|card| card.id)
        .unwrap_or_default();
    let deck = s
        .pile(PlayerId::P1, "library")
        .into_iter()
        .find(|card| card.def_id == core)
        .map(|card| card.id)
        .unwrap_or_default();
    (s, hand, deck)
}

/// The price a play of `id` reads.
pub(crate) fn drone_price_of(s: &Scenario, id: &str) -> i32 {
    effective_cost(s.state(), s.card(id), CostOptions::default())
}

/// R1563000, R2657002: a Float of 2 makes it cost two less in hand, never in the deck.
pub(crate) fn drone_float_two_less(core: &str, printed: i32) {
    for radiant in [false, true] {
        let (mut s, hand, deck) = drone_floating(core, 2, radiant);
        s.end_turn();
        assert_eq!(s.card(hand.as_str()).cost_mod, -2, "radiant {radiant}");
        assert_eq!(s.card(deck.as_str()).cost_mod, 0, "radiant {radiant}");
        assert_eq!(drone_price_of(&s, &hand), printed - 2, "radiant {radiant}");
    }
}

/// R1563000: the opponent's Float lowers nothing.
pub(crate) fn drone_float_opponent(core: &str, printed: i32) {
    let (mut s, hand, _) = drone_floating(core, 0, false);
    s.end_turn();
    assert_eq!(s.card(hand.as_str()).cost_mod, 0);
    s.state_mut().players.p2.mana.current = 4;
    s.end_turn();
    assert_eq!(s.card(hand.as_str()).cost_mod, 0);
    assert_eq!(drone_price_of(&s, &hand), printed);
}
