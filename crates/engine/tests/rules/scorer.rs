//! Port of `packages/engine/test/scorer.test.ts`.
//!
//! The Zephyrs scorer (SPEC §10.7's scorer bullet, R29, §8 #97; BUILD M3-T7).
//!
//! The candidate pool is pinned: this file registers its own small Core catalog, so every ranking
//! below is a statement about a fixed state and a fixed pool, never about the weights' values
//! (§10.7: "the weights are engine constants, tested against fixed states"). Tuning SCORER_WEIGHTS
//! without changing the order of the priorities must leave this file green.

use std::cmp::Ordering;

use jackioh_engine::subsystems::scorer::{
    ScorePriority, Scored, ScorerOptions, ZEPHYRS_INDEX, candidate_defs, compare_scored,
    projected_board_damage, rank, top_three,
};
use jackioh_engine::testkit::*;

use super::fixtures::harness::{new_game, put as put_with, slot};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

/// The harness's `put(state, defId, ref)` with TS's default `options = {}`.
fn put(state: &mut GameState, def_id: &str, at: ZoneSlot) -> CardInstance {
    put_with(state, def_id, at, json!({}))
}

// ---------------------------------------------------------------------------
// The pinned candidate pool.
// ---------------------------------------------------------------------------

/// TS `def(name, args: DefArgs)`: `args` is TS's `DefArgs` literal (`index`, `cost?` = 1, `type?` =
/// "Unit", `attack?`, `health?`, `keywords?` = [], `radiant?`, `token?`, `set?`).
fn def(name: &str, args: Value) -> CardDef {
    let index = args["index"].clone();
    let cost = args.get("cost").cloned().unwrap_or(json!(1));
    let type_ = args.get("type").cloned().unwrap_or(json!("Unit"));
    let attack = args.get("attack").cloned();
    let health = args.get("health").cloned();
    let keywords = args.get("keywords").cloned().unwrap_or(json!([]));
    let mut base = serde_json::Map::new();
    if type_ == json!("Unit") {
        if let Some(attack) = &attack {
            base.insert("attack".to_string(), attack.clone());
        }
        if let Some(health) = &health {
            base.insert("health".to_string(), health.clone());
        }
    }
    base.insert("keywords".to_string(), keywords.clone());
    base.insert("text".to_string(), json!(name));
    let base = Value::Object(base);
    let radiant = match args.get("radiant") {
        None => base.clone(),
        Some(radiant) => {
            let mut face = serde_json::Map::new();
            if let Some(attack) = radiant.get("attack").cloned().or(attack) {
                face.insert("attack".to_string(), attack);
            }
            if let Some(health) = radiant.get("health").cloned().or(health) {
                face.insert("health".to_string(), health);
            }
            face.insert(
                "keywords".to_string(),
                radiant.get("keywords").cloned().unwrap_or(keywords),
            );
            face.insert("text".to_string(), json!(format!("{name} radiant")));
            Value::Object(face)
        }
    };
    json_as(json!({
        "id": format!("sco-{name}"),
        "index": index,
        "name": format!("{name} (scorer)"),
        "set": args.get("set").cloned().unwrap_or(json!("Core")),
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": args.get("token").cloned().unwrap_or(json!(false)),
        "cost": cost,
        "base": base,
        "radiant": radiant,
    }))
}

/// Charge: the printed signal that a card can hit the hero on the turn it arrives (§6.1).
const CHARGER: &str = "sco-charger";
fn charger() -> CardDef {
    def(
        "charger",
        json!({ "index": "10", "cost": 1, "attack": 4, "health": 4, "keywords": [{ "kind": "Charge" }] }),
    )
}
/// The best stats per mana in the pool, and the control case for every priority test.
const BIG_BODY: &str = "sco-big-body";
fn big_body() -> CardDef {
    def(
        "big-body",
        json!({ "index": "20", "cost": 2, "attack": 9, "health": 9 }),
    )
}
/// Poisonous destroys any unit it damages, whatever its health (§6.1).
const POISON_SNAKE: &str = "sco-poison";
fn poison_snake() -> CardDef {
    def(
        "poison",
        json!({ "index": "30", "cost": 1, "attack": 1, "health": 1, "keywords": [{ "kind": "Poisonous" }] }),
    )
}
/// Lifesteal: the one heal printed on a card face (§6.1, §4.4 step 8).
const HEALER: &str = "sco-healer";
fn healer() -> CardDef {
    def(
        "healer",
        json!({ "index": "40", "cost": 1, "attack": 3, "health": 3, "keywords": [{ "kind": "Lifesteal" }] }),
    )
}
const VANILLA: &str = "sco-vanilla";
fn vanilla() -> CardDef {
    def(
        "vanilla",
        json!({ "index": "50", "cost": 1, "attack": 2, "health": 2 }),
    )
}
/// A spell prints no stats, and its text is not machine-readable, so it scores nothing.
const CHEAP_SPELL: &str = "sco-spell";
fn cheap_spell() -> CardDef {
    def("spell", json!({ "index": "60", "cost": 0, "type": "Spell" }))
}
/// A backrow permanent keeps working after it lands (§3.2).
const TRAP_CARD: &str = "sco-trap";
fn trap_card() -> CardDef {
    def("trap", json!({ "index": "70", "cost": 1, "type": "Trap" }))
}
/// Reborn is a second body from one card.
const REBORNER: &str = "sco-reborn";
fn reborner() -> CardDef {
    def(
        "reborn",
        json!({ "index": "80", "cost": 2, "attack": 2, "health": 2, "keywords": [{ "kind": "Reborn" }] }),
    )
}
/// Charge the viewer cannot afford in these states, so it never makes lethal available.
const BIG_CHARGER: &str = "sco-big-charger";
fn big_charger() -> CardDef {
    def(
        "big-charger",
        json!({ "index": "90", "cost": 5, "attack": 9, "health": 9, "keywords": [{ "kind": "Charge" }] }),
    )
}
/// #97 radiant: the picks are radiant, so a radiant-only body has to be scored on that face (§5.2).
const SLEEPER: &str = "sco-sleeper";
fn sleeper() -> CardDef {
    def(
        "sleeper",
        json!({ "index": "85", "cost": 1, "attack": 0, "health": 1, "radiant": { "attack": 20, "health": 20 } }),
    )
}

/// R29: the scorer never offers Zephyrs itself.
const ZEPHYRS: &str = "sco-zephyrs";
fn zephyrs() -> CardDef {
    def(
        "zephyrs",
        json!({ "index": ZEPHYRS_INDEX, "cost": 0, "type": "Spell" }),
    )
}
/// §5.1: `query` never returns a token.
const TOKEN: &str = "sco-token";
fn token() -> CardDef {
    def(
        "token",
        json!({ "index": "T-scorer", "cost": 1, "attack": 9, "health": 9, "token": true }),
    )
}
/// R29: Core only.
const OFF_SET: &str = "sco-off-set";
fn off_set() -> CardDef {
    def(
        "off-set",
        json!({ "index": "1", "cost": 1, "attack": 9, "health": 9, "set": "Classic" }),
    )
}

/// A 12-health body for the enemy board: only Poisonous answers it from printed data (§6.1).
const WALL: &str = "sco-wall";
fn wall() -> CardDef {
    def(
        "wall",
        json!({ "index": "95", "cost": 1, "attack": 2, "health": 12 }),
    )
}

fn pool() -> Vec<CardDef> {
    vec![
        charger(),
        big_body(),
        poison_snake(),
        healer(),
        vanilla(),
        cheap_spell(),
        trap_card(),
        reborner(),
        big_charger(),
        sleeper(),
    ]
}

fn defs() -> Vec<CardDef> {
    let mut defs = pool();
    defs.extend([zephyrs(), token(), off_set()]);
    defs
}

/// A fresh game whose catalog is exactly this file's pool, so a ranking is fully pinned.
fn game(seed: &str, extra: &[CardDef]) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = CardDefs::new();
    for entry in defs().into_iter().chain(extra.iter().cloned()) {
        catalog.insert(entry.id.clone(), entry);
    }
    register_catalog(catalog);
    state.turn = 3;
    state.active = P1;
    state
}

fn ids(scored: &[Scored]) -> Vec<String> {
    scored.iter().map(|entry| entry.def.id.clone()).collect()
}

fn no_options() -> ScorerOptions {
    ScorerOptions::default()
}

fn radiant_picks() -> ScorerOptions {
    ScorerOptions { radiant: Some(true) }
}

fn index_of(list: &[String], id: &str) -> usize {
    list.iter()
        .position(|entry| entry == id)
        .unwrap_or_else(|| panic!("{id} is not ranked"))
}

mod the_zephyrs_scorer_r29_m3_t7 {
    use super::*;

    #[test]
    fn r29_ranks_every_non_token_core_definition_except_97_itself() {
        let state = game("scorer-pool", &[]);

        let mut candidates: Vec<String> = candidate_defs().iter().map(|entry| entry.id.clone()).collect();
        candidates.sort();
        let mut expected: Vec<String> = pool().iter().map(|entry| entry.id.clone()).collect();
        expected.sort();
        assert_eq!(candidates, expected);
        let ranked = rank(&state, P1, &no_options());
        assert_eq!(ranked.len(), pool().len());
        assert!(!ids(&ranked).contains(&ZEPHYRS.to_string()));
        assert!(!ids(&ranked).contains(&TOKEN.to_string()));
        assert!(!ids(&ranked).contains(&OFF_SET.to_string()));
    }

    #[test]
    fn r29_is_deterministic_the_same_state_always_produces_the_same_order() {
        let mut state = game("scorer-determinism", &[]);
        put(&mut state, BIG_BODY, slot(P2, Row::Units, 3));
        state.players.p1.hero.health = 7;
        state.players.p1.mana.current = 3;

        let first = ids(&rank(&state, P1, &no_options()));
        assert_eq!(ids(&rank(&state, P1, &no_options())), first);
        assert_eq!(ids(&rank(&state, P1, &no_options())), first);
        // §9.3: the state is JSON, so a round-tripped copy is the same state and ranks the same.
        assert_eq!(ids(&rank(&clone_state(&state), P1, &no_options())), first);
        // A total order is a property of the pool, not of the seat that asked.
        assert_eq!(ids(&rank(&state, P2, &no_options())).len(), pool().len());
    }

    #[test]
    fn r29_returns_a_total_order_no_two_candidates_tie_whatever_the_weights_are() {
        let state = game("scorer-total-order", &[]);
        let ranked = rank(&state, P1, &no_options());

        let unique: IndexSet<String> = ids(&ranked).into_iter().collect();
        assert_eq!(unique.len(), ranked.len());
        for i in 1..ranked.len() {
            let before = &ranked[i - 1];
            let after = &ranked[i];
            assert!(before.score >= after.score);
            assert_eq!(compare_scored(before, after), Ordering::Less);
            assert_eq!(compare_scored(after, before), Ordering::Greater);
        }

        // The order comes from the comparator, not from the order the catalog handed the pool over.
        let mut shuffled: Vec<Scored> = ranked.clone();
        shuffled.reverse();
        shuffled.sort_by(compare_scored);
        assert_eq!(ids(&shuffled), ids(&ranked));
    }

    #[test]
    fn r29_ranks_a_card_that_enables_lethal_first_and_only_while_the_viewer_can_pay_for_it() {
        let mut state = game("scorer-lethal", &[]);
        // 2 damage on the board, 6 health on the enemy hero: the 4-attack Charge body closes the gap.
        put(&mut state, VANILLA, slot(P1, Row::Units, 1));
        state.players.p2.hero.health = 6;
        state.players.p1.mana.current = 4;

        assert_eq!(projected_board_damage(&state, P1), 2);

        let ranked = rank(&state, P1, &no_options());
        assert_eq!(ranked[0].def.id, CHARGER);
        assert_eq!(ranked[0].priority, ScorePriority::Lethal);
        // The 5-cost Charge body would also be lethal, but 4 mana cannot pay for it.
        let lethal: Vec<String> = ranked
            .iter()
            .filter(|entry| entry.priority == ScorePriority::Lethal)
            .map(|entry| entry.def.id.clone())
            .collect();
        assert_eq!(lethal, vec![CHARGER.to_string()]);
        assert_eq!(top_three(&state, P1, &no_options())[0].def.id, CHARGER);

        // With no mana nothing is lethal, and the pool falls back to stats per mana.
        state.players.p1.mana.current = 0;
        let broke = rank(&state, P1, &no_options());
        assert!(broke.iter().all(|entry| entry.priority != ScorePriority::Lethal));
        assert_eq!(broke[0].def.id, BIG_BODY);

        // Armor on the enemy hero takes the swing back out of lethal range (§4.4 step 2, R44).
        state.players.p1.mana.current = 4;
        state.players.p2.hero.armor = 2;
        assert_eq!(projected_board_damage(&state, P1), 0);
        assert!(
            rank(&state, P1, &no_options())
                .iter()
                .all(|entry| entry.priority != ScorePriority::Lethal)
        );
    }

    #[test]
    fn r29_ranks_clearing_the_enemy_board_above_stats_per_mana() {
        let mut state = game("scorer-clear", &[wall()]);
        put(&mut state, WALL, slot(P2, Row::Units, 2));

        let ranked = rank(&state, P1, &no_options());
        assert_eq!(ranked[0].def.id, POISON_SNAKE);
        assert_eq!(ranked[0].priority, ScorePriority::Clear);
        // The biggest body in the pool cannot answer a 12-health unit, so it is back on the fallback.
        let big = ranked
            .iter()
            .find(|entry| entry.def.id == BIG_BODY)
            .expect("the big body is ranked");
        assert_eq!(big.priority, ScorePriority::Value);
        let order = ids(&ranked);
        assert!(index_of(&order, POISON_SNAKE) < index_of(&order, BIG_BODY));
    }

    #[test]
    fn r29_gives_partial_credit_when_a_card_answers_only_part_of_the_enemy_board() {
        let mut state = game("scorer-partial-clear", &[wall()]);
        put(&mut state, WALL, slot(P2, Row::Units, 2));
        put(&mut state, WALL, slot(P2, Row::Units, 3));

        let ranked = rank(&state, P1, &no_options());
        let poison = ranked
            .iter()
            .find(|entry| entry.def.id == POISON_SNAKE)
            .expect("the snake is ranked");
        let plain = ranked
            .iter()
            .find(|entry| entry.def.id == VANILLA)
            .expect("the vanilla body is ranked");

        // One kill out of two is not a clear, but it still beats a body that answers nothing.
        assert_eq!(poison.priority, ScorePriority::Value);
        assert!(poison.parts.kills > 0.0);
        assert_eq!(poison.parts.clear, 0.0);
        assert_eq!(plain.parts.kills, 0.0);
        let order = ids(&ranked);
        assert!(index_of(&order, POISON_SNAKE) < index_of(&order, VANILLA));
    }

    #[test]
    fn r29_ranks_a_heal_first_only_while_the_viewers_hero_is_below_10() {
        let mut state = game("scorer-heal", &[]);
        state.players.p1.hero.health = SCORER_LOW_HEALTH - 1;

        let low = rank(&state, P1, &no_options());
        assert_eq!(low[0].def.id, HEALER);
        assert_eq!(low[0].priority, ScorePriority::Heal);

        // At exactly 10 the hero is not below 10, so the heal is worth no more than its body.
        state.players.p1.hero.health = SCORER_LOW_HEALTH;
        let fine = rank(&state, P1, &no_options());
        assert!(fine.iter().all(|entry| entry.priority != ScorePriority::Heal));
        assert_eq!(fine[0].def.id, BIG_BODY);
    }

    #[test]
    fn r29_falls_back_to_stats_per_mana_plus_draw_value_and_the_discover_offers_the_top_3() {
        let state = game("scorer-top-three", &[]);
        let ranked = rank(&state, P1, &no_options());

        // A fixed, empty state: nothing is lethal, there is no enemy board and the hero is at full.
        assert!(ranked.iter().all(|entry| entry.priority == ScorePriority::Value));
        assert_eq!(
            ids(&ranked),
            vec![
                BIG_BODY,     // 18 stats for 2 mana
                CHARGER,      // 8 stats for 1 mana
                HEALER,       // 6 stats for 1 mana
                REBORNER,     // 4 stats for 2 mana, plus a second body
                VANILLA,      // 4 stats for 1 mana
                BIG_CHARGER,  // 18 stats for 5 mana
                POISON_SNAKE, // 2 stats for 1 mana
                TRAP_CARD,    // no stats, but it stays on the board
                SLEEPER,      // 1 stat for 1 mana on its base face
                CHEAP_SPELL,  // no stats, and its text is not machine-readable
            ]
        );

        let three = top_three(&state, P1, &no_options());
        assert_eq!(three.len(), 3);
        assert_eq!(ids(&three), vec![BIG_BODY, CHARGER, HEALER]);
        assert_eq!(ids(&three), ids(&ranked[..3]));
    }

    #[test]
    fn spec_8_97_radiant_scores_the_radiant_face_of_each_candidate() {
        let state = game("scorer-radiant", &[]);

        assert_eq!(rank(&state, P1, &no_options())[0].def.id, BIG_BODY);
        // The sleeper is a 0/1 that prints a 20/20 radiant face, so radiant picks put it first (§5.2).
        assert_eq!(rank(&state, P1, &radiant_picks())[0].def.id, SLEEPER);
        assert_eq!(
            ids(&top_three(&state, P1, &radiant_picks())),
            vec![SLEEPER, BIG_BODY, CHARGER]
        );
    }
}
