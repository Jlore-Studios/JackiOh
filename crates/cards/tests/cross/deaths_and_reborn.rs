//! Deaths, their killers and Reborn bodies (SPEC §4.5, §6.1, §6.3 Sacrifice, R42, R78, R83, R89,
//! R174). Found by the polish-4 edge-case hunt, round 2 (docs/polish/4-edge-cases.md, lenses L2, L3
//! and L8); every case here failed before its fix.
//!
//!  - §4.5 step 1, R89: the units one check collects are read before any of them moves, so each dies
//!    with the aura and the layer-2 stats it had — not with lane order deciding which it lost.
//!  - R42, R89: the killer is the hit that took the unit to 0, credited as it lands, so a unit an
//!    aura later starves has no killer.
//!  - R174: what a card queued while it stood on the field, and a delayed effect aimed at it, end
//!    with that stay, so a Reborn body is not acted on by either.
//!  - Round 6, lens L2. §4.5 step 4, §10.1: Reborn returns a collected unit from the graveyard step 1
//!    moved it to, and from nowhere else, so a Death hook of the same pass that moved it on (a later
//!    set's "exile your graveyard", built here as a fixture) does not leave it in two zones.
//!  - Round 9, lens "combat windows". R42, R89: a unit is killed once, by the first thing that dooms
//!    it before the check collects it, and what lands on it afterwards changes nothing: a destroy after
//!    the lethal hit, a Poisonous hit on a unit already at 0, a hit on a unit a Poisonous hit already
//!    marked. The cards are fixtures (no Core Death deals damage, and no Core Cry both damages and
//!    destroys; #99 crafting #68 with #2 is the Core shape of the first).
//!
//! Port of `packages/cards/test/deaths-and-reborn.test.ts` (SURFACE §4.1, §8). TS's live card objects
//! are owned copies here, read back from the state by id (`g.card(&id)`) after every step and written
//! through `find_instance_mut`.

use jackioh_engine::effects::{damage, destroy, draw, exile_matching};
use jackioh_engine::testkit::*;
use jackioh_engine::PlayerId::{P1, P2};

const BIG_D: &str = "core-001";
const STOCKPILE: &str = "core-005";
const VANILLA: &str = "core-008";
const MOTHS: &str = "core-009";
const TIMMY: &str = "core-011";
const HIT_JOB: &str = "core-016";
const HINDER: &str = "core-021";
const PANTHER: &str = "core-032";
const GRAVEDIGGER: &str = "core-037";
const BIG_FELINOR: &str = "core-043";
const TRUE_STRIKE: &str = "core-044";
const SUPPRESSIVE: &str = "core-046";
const KPOP: &str = "core-050";
const SURGERY: &str = "core-063";
const PILLOW: &str = "core-065-1";
const CORPSE_EATER: &str = "core-089";
const FAUCI: &str = "core-091";
const FIENDER: &str = "core-092";
const LIBRARY: [&str; 10] = [VANILLA; 10];

/// On these seeds #63 Plastic Surgery's first random keyword is Reborn: for a unit that already has
/// Rush (Fed Fauci) the pool order and the first rng draw match re-entry.test.ts's token, so it shares
/// that seed; the Gravedigger scenario draws from a fuller pool at another rng cursor, so it carries
/// its own.
const REBORN_SEED: &str = "re-entry-reborn-token-32"; // R346 put Pierce in the pool, which moved the roll off "-4"; R636's Windfury moved it off "-10" and "-19"; R49's Deft moved it off "-31".
const REBORN_SEED_DIGGER: &str = "re-entry-reborn-token-40"; // Same history: R49's Deft moved the roll off "-31".

use super::scenario;

/// TS `at(card)`: the one-instance target list a play sends.
fn at(card: &CardInstance) -> Value {
    json!([{ "pick": "instance", "instanceId": card.id }])
}

fn unit_at(g: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
    match g.unit(player, lane) {
        Some(card) => card.clone(),
        None => panic!("setup: {player} should hold a unit in lane {lane}"),
    }
}

fn keywords_of(g: &Scenario, card: &CardInstance) -> Vec<KeywordKind> {
    g.stats(&card.id).keywords.iter().map(Keyword::kind).collect()
}

/// What a `destroyed` event says the unit was as it died (R89).
struct Died {
    attack: i32,
    max_health: i32,
}

fn destroyed_of(g: &Scenario, card: &CardInstance) -> Died {
    let found = g.events().iter().find_map(|event| match event {
        GameEvent::Destroyed { instance_id, attack, max_health, .. } if *instance_id == card.id => Some(Died {
            attack: *attack,
            max_health: *max_health,
        }),
        _ => None,
    });
    match found {
        Some(died) => died,
        None => panic!("no destroyed event for {}", card.id),
    }
}

fn was_destroyed(events: &[GameEvent], id: &str) -> bool {
    events
        .iter()
        .any(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if instance_id == id))
}

mod r89_4_5_step_1_collects_at_once_so_every_unit_dies_as_it_stood {
    use super::*;

    #[test]
    fn r89_r38_a_unit_that_dies_beside_the_spikey_pillow_draining_it_dies_with_the_pillows_2_attack() {
        let mut g = scenario(json!({
            "p1": { "hand": [{ "def": HIT_JOB, "radiant": true }, CORPSE_EATER, HINDER], "library": LIBRARY },
            "p2": { "hand": [HINDER], "field": [{ "def": PILLOW, "lane": 1 }, { "def": VANILLA, "lane": 2 }], "library": LIBRARY },
        }));
        let vanilla = unit_at(&g, P2, 2);
        assert_eq!(vanilla.def_id, VANILLA);
        // Before: the Pillow's aura drains p2's Mr. Vanilla to 2 attack (§10.4 layer 5).
        assert_eq!(g.stats(&vanilla.id).attack, 2);
        assert_eq!(g.stats(&vanilla.id).max_health, 4);

        // Radiant Hit Job on the Vanilla also destroys the Pillow beside it: both are collected in one
        // pass and moved "all … at once" (§4.5 step 1).
        g.play(HIT_JOB, json!({ "targets": at(&vanilla) }));
        g.expect_in_zone(&vanilla.id, "graveyard");

        // R89: the event carries what the unit was as it died — with the Pillow still beside it.
        let died = destroyed_of(&g, &vanilla);
        assert_eq!(died.attack, 2);
        assert_eq!(died.max_health, 4);
        // R38: Corpse Eater gains the dying unit's current attack and max health (the Pillow is a
        // token and never feeds it, R11).
        assert_eq!(g.card(CORPSE_EATER).buffs, AttackHealth { attack: 2, health: 4 });
    }

    #[test]
    fn r89_r38_felinor_fiender_dying_with_a_felinor_beside_it_dies_with_that_felinors_stats_in_its_own_10_4_layer_2() {
        let mut g = scenario(json!({
            "p1": { "hand": [{ "def": HIT_JOB, "radiant": true }, CORPSE_EATER, HINDER], "library": LIBRARY },
            "p2": { "hand": [HINDER], "field": [{ "def": BIG_FELINOR, "lane": 1 }, { "def": FIENDER, "lane": 2 }], "library": LIBRARY },
        }));
        let fiender = unit_at(&g, P2, 2);
        assert_eq!(fiender.def_id, FIENDER);
        // 5/7 printed, plus Big Felinor's 3/10.
        assert_eq!(g.stats(&fiender.id).attack, 8);
        assert_eq!(g.stats(&fiender.id).max_health, 17);

        g.play(HIT_JOB, json!({ "targets": at(&fiender) }));
        g.expect_in_zone(&fiender.id, "graveyard");
        let felinor = g.card(BIG_FELINOR).id.clone();
        g.expect_in_zone(&felinor, "graveyard");

        let died = destroyed_of(&g, &fiender);
        assert_eq!(died.attack, 8);
        assert_eq!(died.max_health, 17);
        // Big Felinor's 3/10 plus the Fiender's 8/17.
        assert_eq!(g.card(CORPSE_EATER).buffs, AttackHealth { attack: 11, health: 27 });
    }
}

mod r42_the_killer_is_the_hit_that_took_the_unit_to_0 {
    use super::*;

    #[test]
    fn r42_r89_a_unit_prem_panther_hit_earlier_that_an_aura_later_kills_was_not_destroyed_by_the_panther() {
        // 5 damage on a 0/7 Big D-fender leaves it at 2. Radiant Suppressive Aura paid 2 then gives
        // enemy units -2/-2: max health 5 under 5 damage, so it dies — to the aura, which is no damage
        // instance, not to a hit that left it standing.
        let mut s = scenario(json!({
            "seed": "hunt-cw2-panther-aura",
            "p1": { "field": [PANTHER], "hand": [{ "def": SUPPRESSIVE, "radiant": true }, STOCKPILE], "library": [TIMMY, TIMMY, TIMMY] },
            "p2": { "field": [BIG_D], "hand": [STOCKPILE] },
        }));
        let dfender = s.card(BIG_D).clone();

        s.attack(PANTHER, &dfender.id);
        s.expect_stats(&dfender.id, json!({ "health": 2 }));
        let hand_before = s.hand(P1).len();

        s.play(SUPPRESSIVE, json!({ "embiggen": false }));

        s.expect_in_zone(&dfender.id, "graveyard");
        let destroyed = s.last_events().iter().find_map(|event| match event {
            GameEvent::Destroyed { instance_id, killer_id, .. } if *instance_id == dfender.id => Some(killer_id.clone()),
            _ => None,
        });
        // toMatchObject({ killerId: null }): the event is there, and names no killer.
        assert_eq!(destroyed, Some(None));
        // §8 #32: "whenever this destroys a unit, draw 2" — it did not, so nothing was drawn.
        assert_eq!(s.hand(P1).len(), hand_before - 1);
    }
}

mod r174_what_was_queued_for_a_cards_old_stay_does_not_act_on_its_reborn_body {
    use super::*;

    #[test]
    fn r174_r78_fed_faucis_plague_counter_for_the_hit_that_killed_it_does_not_land_on_its_reborn_body() {
        let mut g = scenario(json!({
            "seed": REBORN_SEED,
            "p1": {
                "hand": [SURGERY, TRUE_STRIKE, VANILLA],
                "field": [{ "def": FAUCI, "lane": 1, "damage": 5 }],
                "library": LIBRARY,
            },
            "p2": { "hand": [VANILLA], "library": LIBRARY },
        }));
        let fauci = g.card(FAUCI).clone();
        g.play(SURGERY, json!({ "targets": at(&fauci) }));
        assert!(keywords_of(&g, &fauci).contains(&KeywordKind::Reborn));

        // A 4/9 with 5 damage: True Strike's 4 is lethal, and Reborn brings the same instance back.
        g.play(TRUE_STRIKE, json!({ "targets": at(&fauci) }));
        assert!(was_destroyed(g.events(), &fauci.id));
        g.expect_in_zone(&fauci.id, "field");
        assert_eq!(g.card(&fauci.id).reborn_spent, Some(true));
        // R78: counters reset as it left, and the body that came back has taken no damage.
        assert_eq!(g.card(&fauci.id).counters.plague.unwrap_or(0), 0);
    }

    #[test]
    fn r174_r83_a_gravedigger_that_dies_to_cleave_during_moths_run_and_comes_back_takes_no_card_from_its_start_of_turn_hook() {
        let mut g = scenario(json!({
            "seed": REBORN_SEED_DIGGER,
            "p1": {
                "hand": [SURGERY, VANILLA],
                "field": [{ "def": MOTHS, "lane": 1 }, { "def": GRAVEDIGGER, "lane": 2 }],
                "graveyard": [VANILLA],
                "library": LIBRARY,
            },
            "p2": { "hand": [VANILLA], "field": [{ "def": PANTHER, "radiant": true, "lane": 1 }], "library": LIBRARY },
        }));
        let digger = g.card(GRAVEDIGGER).clone();
        g.play(SURGERY, json!({ "targets": at(&digger) }));
        assert!(keywords_of(&g, &digger).contains(&KeywordKind::Reborn));
        g.end_turn(); // p2's turn
        g.end_turn(); // p1's start of turn: Moths (lane 1) then Gravedigger (lane 2) are queued (R68)
        assert_eq!(g.state().active, P1);

        // Moths forced the radiant Panther to attack it; its Cleave killed Gravedigger in lane 2, and
        // Gravedigger came back through Reborn before its own queued hook popped.
        assert!(was_destroyed(g.events(), &digger.id));
        g.expect_in_zone(&digger.id, "field");
        // The body entered after the turn started (R83), so the start of turn was not its to answer:
        // nothing leaves p1's graveyard.
        let mut left: Vec<String> = g.pile(P1, "graveyard").iter().map(|card| card.def_id.clone()).collect();
        left.sort();
        let mut expected = vec![SURGERY.to_string(), VANILLA.to_string()];
        expected.sort();
        assert_eq!(left, expected);
    }

    #[test]
    fn r174_r76_a_delayed_steal_whose_target_died_and_came_back_through_reborn_during_an_earlier_delayed_effect_fizzles() {
        // p1 plays two K-Pop Fanatics on turn 9: A on p2's Big Felinor, then B on p2's Felinor Fiender,
        // which has Reborn and 10 damage and stands at 8/17 only because Big Felinor feeds its stats
        // (§8 #92, R116). At p1's next start of turn A steals Big Felinor first (R68's creation order),
        // the Fiender drops to 5/7 and dies, and Reborn brings it straight back at 1 health. B's target
        // left the field in between, so B fizzles (R76) and the Reborn body stays p2's.
        let mut g = scenario(json!({
            "p1": { "hand": [KPOP, KPOP, VANILLA], "library": LIBRARY },
            "p2": {
                "hand": [VANILLA],
                "field": [
                    { "def": FIENDER, "lane": 1, "damage": 10 },
                    { "def": BIG_FELINOR, "lane": 2 },
                ],
                "library": LIBRARY,
            },
        }));
        let fiender = unit_at(&g, P2, 1);
        let felinor = unit_at(&g, P2, 2);
        if let Some(card) = g.state_mut().players.p2.units[0].as_mut().and_then(|pile| pile.first_mut()) {
            card.granted_keywords.push(Keyword::Reborn);
        }
        assert_eq!(g.stats(&fiender.id).health, 7);

        let kpops: Vec<CardInstance> = g.hand(P1).iter().filter(|card| card.def_id == KPOP).cloned().collect();
        let (Some(kpop_a), Some(kpop_b)) = (kpops.first(), kpops.get(1)) else {
            panic!("setup: two K-Pop Fanatics in hand");
        };
        g.play(&kpop_a.id, json!({ "targets": [{ "pick": "instance", "instanceId": felinor.id }] }));
        g.play(&kpop_b.id, json!({ "targets": [{ "pick": "instance", "instanceId": fiender.id }] }));
        g.end_turn();
        assert_eq!(g.state().active, P2);
        g.end_turn();
        assert_eq!(g.state().active, P1);

        // A landed; the Fiender died and came back.
        assert_eq!(g.card(&felinor.id).controller, P1);
        assert!(was_destroyed(g.events(), &fiender.id));
        g.expect_in_zone(&fiender.id, "field");
        assert_eq!(g.card(&fiender.id).reborn_spent, Some(true));

        // B fizzled: the body that came back is still p2's.
        assert_eq!(g.card(&fiender.id).controller, P2);
    }
}

const RENO: &str = "core-053";
const RADIANT_SAINTESS: &str = "core-081";
const TWISTING_NETHER: &str = "core-088";

fn must<T>(value: Option<T>, what: &str) -> T {
    match value {
        Some(value) => value,
        None => panic!("missing: {what}"),
    }
}

/// A fixture card: a transient def in the match state and its script in the registry. TS
/// `fixture(s, id, type, script, stats = { attack: 2, health: 2 })`.
fn fixture(s: &mut Scenario, id: &str, type_: CardType, script: Script, stats: Option<AttackHealth>) {
    let stats = stats.unwrap_or(AttackHealth { attack: 2, health: 2 });
    let face = if type_ == CardType::Unit {
        json!({ "attack": stats.attack, "health": stats.health, "keywords": [], "text": id })
    } else {
        json!({ "keywords": [], "text": id })
    };
    let def: CardDef = json_as(json!({
        "id": id,
        "index": id,
        "name": id,
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face.clone(),
        "radiant": face,
    }));
    s.state_mut().transient_defs.insert(id.to_string(), def);
    let mut scripts = registered_scripts().clone();
    scripts.insert(id.to_string(), CardScripts { base: script.clone(), radiant: script });
    register_scripts(scripts);
}

fn place_fixture(s: &mut Scenario, def_id: &str, player: PlayerId, row: Row, lane: i32) -> CardInstance {
    let mut card = new_instance(s.state_mut(), def_id, player, Zone::Hand { player });
    if !place_on_field(s.state_mut(), &mut card, &ZoneSlot { player, row, lane }, Default::default()) {
        panic!("could not place {def_id}");
    }
    let turn = s.state().turn;
    let live = s.card_mut(&card.id);
    live.summoned_turn = Some(turn - 1);
    live.clone()
}

/// Every pile of the state that holds this instance id (§10.1: a card is in one zone at a time).
fn piles_holding(state: &GameState, id: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for player in PLAYER_IDS {
        let side = &state.players[player];
        for (zone, pile) in [
            ("hand", &side.hand),
            ("library", &side.library),
            ("graveyard", &side.graveyard),
            ("exile", &side.exile),
            ("resolving", &side.resolving),
        ] {
            if pile.iter().any(|card| card.id == id) {
                out.push(format!("{player}.{zone}"));
            }
        }
        for (at, pile) in side.units.iter().enumerate() {
            if pile.as_ref().is_some_and(|pile| pile.iter().any(|card| card.id == id)) {
                out.push(format!("{player}.units.{}", at + 1));
            }
        }
        for (at, card) in side.backrow.iter().enumerate() {
            if card.as_ref().is_some_and(|card| card.id == id) {
                out.push(format!("{player}.backrow.{}", at + 1));
            }
        }
    }
    out
}

mod section_4_5_step_4_10_1_reborn_never_leaves_one_card_in_two_zones {
    use super::*;

    #[test]
    fn r78_r127_4_5_a_reborn_unit_a_death_hook_exiled_out_of_the_graveyard_is_not_also_put_back_on_the_field() {
        let mut g = scenario(json!({
            "p1": { "hand": [TWISTING_NETHER, STOCKPILE], "field": [{ "def": RADIANT_SAINTESS, "lane": 1 }] },
            "p2": { "hand": [STOCKPILE], "field": [{ "def": RENO, "lane": 1 }] },
        }));
        // "Death: exile your graveyard", the way a later set's grave-robber would print it.
        fixture(
            &mut g,
            "edge-r6-grave-robber",
            CardType::Unit,
            Script {
                death: Some(hook(|_ctx| vec![exile_matching(json_as(json!({ "zones": ["graveyard"] })))])),
                ..Script::default()
            },
            None,
        );
        place_fixture(&mut g, "edge-r6-grave-robber", P1, Row::Units, 2);
        let saintess = must(g.unit(P1, 1), "Radiant Saintess");

        // Twisting Nether destroys both. §4.5 step 3 runs the grave-robber's Death in lane order after
        // the Saintess's, and it exiles her from the graveyard she was collected to, where step 4 would
        // have found her.
        g.play(TWISTING_NETHER, json!({}));

        // Wherever she ends up — back on the field or left in exile — she is one card in one zone, and
        // her `zone` says which.
        let zone = g.card(&saintess.id).zone.clone();
        let where_ = match &zone {
            Zone::Field { player, row, lane } => format!("{player}.{row}.{lane}"),
            other => format!("{}.{}", other.player(), other.z()),
        };
        assert_eq!(piles_holding(g.state(), &saintess.id), vec![where_]);
    }
}

// ---------------------------------------------------------------------------
// Round 9: a unit already killed is not killed again (R42, R89, §4.4 step 7)
// ---------------------------------------------------------------------------

/// A fixture unit with a face of its own: a transient def in the match state and its script in the
/// registry. TS `fixtureUnit(s, id, script, stats, keywords = [])`.
fn fixture_unit(s: &mut Scenario, id: &str, script: Script, stats: AttackHealth, keywords: Vec<Keyword>) {
    let face = json!({ "attack": stats.attack, "health": stats.health, "keywords": keywords, "text": id });
    let def: CardDef = json_as(json!({
        "id": id,
        "index": id,
        "name": id,
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face.clone(),
        "radiant": face,
    }));
    s.state_mut().transient_defs.insert(id.to_string(), def);
    let mut scripts = registered_scripts().clone();
    scripts.insert(id.to_string(), CardScripts { base: script.clone(), radiant: script });
    register_scripts(scripts);
}

fn in_hand(s: &mut Scenario, def_id: &str, player: PlayerId) -> CardInstance {
    let card = new_instance(s.state_mut(), def_id, player, Zone::Hand { player });
    s.state_mut().players[player].hand.push(card.clone());
    card
}

fn place_unit(s: &mut Scenario, def_id: &str, player: PlayerId, lane: i32) -> CardInstance {
    let mut card = new_instance(s.state_mut(), def_id, player, Zone::Hand { player });
    if !place_on_field(s.state_mut(), &mut card, &ZoneSlot { player, row: Row::Units, lane }, Default::default()) {
        panic!("could not place {def_id}");
    }
    let live = s.card_mut(&card.id);
    live.position = Some(Position::Atk);
    live.clone()
}

/// TS `killerOf`: `None` when the unit has no `destroyed` event (TS `undefined`), else its killer.
fn killer_of(s: &Scenario, card: &CardInstance) -> Option<Option<String>> {
    s.events().iter().find_map(|event| match event {
        GameEvent::Destroyed { instance_id, killer_id, .. } if *instance_id == card.id => Some(killer_id.clone()),
        _ => None,
    })
}

fn one_unit() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

/// A board for one Death pass: p1's 3/3 Mr. Vanilla in lane 3, and p1 holding a radiant Hit Job that
/// destroys p2's lane-1 unit and the lane-2 unit beside it, so both of their Deaths run in that one
/// pass, lane 1 first (R68), with no state check between them (§4.5 step 3).
fn death_pass_board() -> (Scenario, CardInstance) {
    let s = scenario(json!({
        "p1": { "hand": [{ "def": HIT_JOB, "radiant": true }, RENO], "field": [{ "def": VANILLA, "lane": 3 }], "library": LIBRARY },
        "p2": { "hand": [RENO], "library": LIBRARY },
    }));
    let vanilla = must(s.unit(P1, 3), "p1's Mr. Vanilla");
    (s, vanilla)
}

/// "Death: deal `amount` damage to p1's Mr. Vanilla."
fn hits_vanilla(vanilla: &CardInstance, amount: i32) -> Script {
    let id = vanilla.id.clone();
    Script {
        death: Some(hook(move |_ctx| {
            vec![damage(json_as(json!({ "to": { "of": "instance", "instanceId": id }, "amount": amount })))]
        })),
        ..Script::default()
    }
}

fn hit_landed(s: &Scenario, source: &CardInstance, target: &CardInstance, how_much: i32) -> bool {
    s.events().iter().any(|event| {
        matches!(event, GameEvent::Damage { source_id, target_id, amount, .. }
            if source_id.as_deref() == Some(source.id.as_str()) && *target_id == target.id && *amount == how_much)
    })
}

mod r42_r89_a_unit_already_killed_is_not_killed_again {
    use super::*;

    #[test]
    fn r42_r89_a_destroy_that_lands_after_this_units_lethal_hit_leaves_the_kill_with_this_unit_so_its_destroys_a_unit_text_draws_2() {
        let mut s = scenario(json!({
            "p1": { "hand": [RENO], "library": LIBRARY },
            "p2": { "field": [{ "def": VANILLA, "lane": 1 }], "hand": [RENO], "library": LIBRARY },
        }));
        // "Cry: deal 5 damage to a target unit; destroy it", carrying a "whenever this destroys a Unit,
        // draw 2" text that reads R42's killer off the death — what #32's text was before R426 tied it to
        // the Panther's own attacks.
        let kill_draws = TriggerDef::new("edge-r9-maul-kill-draws", &[GameEventType::Destroyed], |ctx, event| {
            let Some(me) = ctx.self_.as_ref() else {
                return vec![];
            };
            match event {
                GameEvent::Destroyed { killer_id, .. } if killer_id.as_deref() == Some(me.id.as_str()) => {
                    vec![draw(json_as(json!({ "count": 2 })))]
                }
                _ => vec![],
            }
        });
        fixture_unit(
            &mut s,
            "edge-r9-maul",
            Script {
                targets: one_unit(),
                triggers: vec![kill_draws],
                cry: Some(hook(|_ctx| {
                    vec![
                        damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 5 }))),
                        destroy(json_as(json!({ "target": { "of": "chosen" } }))),
                    ]
                })),
                ..Script::default()
            },
            AttackHealth { attack: 2, health: 2 },
            vec![],
        );
        let maul = in_hand(&mut s, "edge-r9-maul", P1);
        let vanilla = must(s.unit(P2, 1), "p2's Mr. Vanilla");
        let hand = s.hand(P1).len();

        s.play(&maul.id, json!({ "targets": [{ "pick": "instance", "instanceId": vanilla.id }] }));

        // The 5 took the 3/3 to -2: that hit was the lethal damage instance, and the destroy that
        // followed it found a unit already dead (§4.5 step 1 collects it on either count).
        s.expect_in_zone(&vanilla.id, "graveyard");
        assert_eq!(killer_of(&s, &vanilla), Some(Some(maul.id.clone())));
        // The maul left the hand and its kill drew 2.
        assert_eq!(s.hand(P1).len(), hand - 1 + 2);
    }

    #[test]
    fn r42_r89_a_poisonous_hit_on_a_unit_an_earlier_hit_already_took_to_0_does_not_take_the_kill_from_that_hit_4_4_step_7() {
        let (mut s, vanilla) = death_pass_board();
        // Two p2 units whose Deaths hit p1's 3/3: lane 1's Death deals 5, then (R68 order) lane 2's,
        // a Poisonous unit's, deals 1 — both in the one Death pass, with no check between (§4.5 step 3).
        fixture_unit(&mut s, "edge-r9-hammer", hits_vanilla(&vanilla, 5), AttackHealth { attack: 1, health: 1 }, vec![]);
        fixture_unit(
            &mut s,
            "edge-r9-sting",
            hits_vanilla(&vanilla, 1),
            AttackHealth { attack: 1, health: 1 },
            vec![Keyword::Poisonous],
        );
        let hammer = place_unit(&mut s, "edge-r9-hammer", P2, 1);
        let sting = place_unit(&mut s, "edge-r9-sting", P2, 2);

        // Radiant Hit Job destroys the hammer and the sting beside it.
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": hammer.id }] }));

        assert!(hit_landed(&s, &hammer, &vanilla, 5));
        assert!(hit_landed(&s, &sting, &vanilla, 1));
        s.expect_in_zone(&vanilla.id, "graveyard");
        // The hammer's 5 took it from 3 to -2; the sting hit a unit that was already dead.
        assert_eq!(killer_of(&s, &vanilla), Some(Some(hammer.id.clone())));
    }

    #[test]
    fn r42_r89_a_hit_that_takes_a_unit_a_poisonous_hit_already_marked_destroyed_to_0_does_not_take_the_kill_from_the_poisonous_unit_4_4_step_7()
     {
        let (mut s, vanilla) = death_pass_board();
        // The same pass the other way round: lane 1's Poisonous Death deals 1 (3/3 → 2 health, marked
        // destroyed), then lane 2's deals 5 to a unit that step 7 has already destroyed.
        fixture_unit(
            &mut s,
            "edge-r9-sting",
            hits_vanilla(&vanilla, 1),
            AttackHealth { attack: 1, health: 1 },
            vec![Keyword::Poisonous],
        );
        fixture_unit(&mut s, "edge-r9-hammer", hits_vanilla(&vanilla, 5), AttackHealth { attack: 1, health: 1 }, vec![]);
        let sting = place_unit(&mut s, "edge-r9-sting", P2, 1);
        let hammer = place_unit(&mut s, "edge-r9-hammer", P2, 2);

        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": sting.id }] }));

        assert!(hit_landed(&s, &sting, &vanilla, 1));
        assert!(hit_landed(&s, &hammer, &vanilla, 5));
        s.expect_in_zone(&vanilla.id, "graveyard");
        // "The Poisonous hit is the one that destroys it, whatever health it left" (damage.ts, R42):
        // the sting's hit killed it, and the hammer's 5 landed on a unit already destroyed.
        assert_eq!(killer_of(&s, &vanilla), Some(Some(sting.id.clone())));
    }
}
