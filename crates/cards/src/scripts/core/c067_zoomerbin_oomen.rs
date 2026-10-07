//! #67 Zoomerbin Oomen (SPEC §8.3, §3.1, §3.2, §5.1, R1, R33, R47, R60, R97, R177, R275).
//! Unit, Human, cost 1, 1/2 → 2/4.
//!   Base:    "Cry: Summon a random Cost (1) Trap face-down into your backrow zone in this lane."
//!   Radiant: "Cry: Summon a random Radiant Trap face-down into your backrow zone in this lane." —
//!            §8's cell "A random Radiant Trap". The cell restates only which trap comes: any Trap,
//!            summoned Radiant, which is the face's raise (R275). "Face-down" and "your backrow
//!            zone in this lane" are the base clause's, kept (§8 Conventions).
//!
//! §3.1: "'This lane' (Zoomerbin Oomen) means the backrow zone in the same column as the unit", so
//! the lane is the Cry's own unit's lane, read back with the engine's `slot_of`. `summon` derives the
//! row from the def's type (`row_of`: everything but a Unit and a Spell goes to the backrow), so
//! naming the lane is the whole placement.
//!
//! R47: a lane-targeted summon into an occupied zone fizzles, and §8's Conventions keep the unit on
//! the field regardless — "the unit still enters". A Locked zone takes the summon since balance patch
//! 1 (R688: a Lock refuses only plays; this card's text says nothing of Locks). `summon`'s
//! `zone_for`/`can_place` already implements exactly that (reserved or occupied → no zone → nothing
//! created; Locked → the trap lands), so this card needs no check of its own and must not grow one.
//!
//! §3.2 and R33: `summon_onto` leaves anything that is not a Field Spell face-down, and only the
//! current controller may read a face-down trap. R1: a summon fires no Cry and pays nothing, so the
//! trap arrives unpaid and dormant until its own trigger condition is met.
//!
//! THE POOL. The Engine cell: the base pool is the Cost (1) traps, #18, #41, #60, #71, #96 (patch
//! v0.1.1 made #85 cost 2; balance patch 1 adds Classic #10 Exile at (1)), and the radiant face's is
//! every Core trap, #85 included, summoned Radiant; zone occupied → fizzles. `TRAP_TYPES` is `["Trap", "Field Trap"]` because the
//! filters match `def.type` exactly while SPEC reads "Field Trap counts as Trap" (§8 #51, R35, R61) —
//! `test/query.test.ts` pins both pools. The radiant form drops the cost, keeps the types and sets the
//! §5.2 flag on the trap it makes. §5.1 keeps tokens out
//! of a pool that does not ask for them and orders the result by §5 index, so a seeded pick replays
//! identically (§9.3, R60). A Radiant face-down trap is still a face-down trap: only its controller
//! may read it, its face included (R33, R97, R177), which `view_for` owns.
//!
//! THE VERB. `summon_random({ query, player, lane, radiant })` (engine/src/effects/summon.rs) resolves
//! the pool through `catalog::query`, draws one def with `ctx.rng.pick`, and hands the placement to
//! the same code `summon` uses, so R47's fizzle and the face-down trap stay in one place. A card file
//! must not pick the def itself: that would put randomness in `packages/cards` instead of the effects
//! library (CLAUDE.md rules 4 and 5).

use jackioh_engine::effects::summon_random;
use jackioh_engine::prelude::*;

use crate::query::{CardQuery, TRAP_TYPES};

pub const ID: &str = "core-067";

/// §8.3: the base text names 1-cost traps; §5.1's `cost` reads a def's cost out of play (R65).
const TRAP_COST: i32 = 1;

/// Both pools, as `test/query.test.ts` spells them: the five Cost (1) traps, and all six.
fn one_cost_traps() -> CardQuery {
    json_as(json!({ "type": TRAP_TYPES, "cost": TRAP_COST }))
}

fn any_trap() -> CardQuery {
    json_as(json!({ "type": TRAP_TYPES }))
}

/// The faces differ in the pool and in whether the trap comes Radiant, so one factory takes both.
///
/// §10.9 lets a hook read state to compute an effect's arguments; it never writes. The single read
/// here is the engine's own `slot_of`, which turns the Cry's unit into `{ player, row, lane }`.
fn oomen(pool: CardQuery, radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            // TS read the live `ctx.self`: its zone is what `slot_of` reads.
            let Some(self_) = ctx.live_self() else {
                return vec![];
            };
            let at = slot_of(&ctx.state, self_);
            // Off the field there is no "this lane" to summon into, so nothing happens (§3.1).
            let Some(at) = at else {
                return vec![];
            };
            vec![summon_random(json_as(json!({
                "query": pool,
                "player": "self",
                "lane": at.lane,
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: oomen(one_cost_traps(), false),
        radiant: oomen(any_trap(), true),
    }
}

// #67 Zoomerbin Oomen — SPEC §8.3, BUILD M4-T4: "Random trap face-down and unpaid into own lane's
// backrow; occupied or Locked → nothing (R47); pool = the five Cost (1) traps, radiant all six".
//
// §8.3's row: "Cry: summon a random Cost (1) Trap face-down into your backrow zone in this lane" →
// "A random Radiant Trap" (R275). Patch v0.1.1 made #85 Unlicensed Experimentation cost 2, so the
// base pool is #18, #41, #60, #71, #96 — and since patch v0.2.0's one format (R380) Classic+ #22 —
// and the radiant face's is every trap of every set, #85 included, summoned Radiant; zone occupied or Locked → fizzles. A Radiant face-down trap is still hidden from the opponent, face and all (R33,
// R97, R177).
//
// §3.1 fixes what "this lane" means: "the backrow zone in the same column as the unit". §8's
// Conventions fix the fizzle: the Cry does nothing and "the unit still enters". R1 fixes the
// unpaid, dormant arrival: a summon fires no Cry and pays no mana.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::query::{CardQuery, TRAP_TYPES};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const OOMEN: &str = "core-067"; // Unit 1/2 → 2/4, cost 1, Human.
    const MANA_WELL: &str = "core-006"; // A Field Spell: something to occupy a backrow zone with.
    const SIPHON_SQUAD: &str = "classic-088"; // Field Trap that Tributes itself while the opponent has no Units (R403).

    /// The base face's pool, by catalog id: the Cost (1) traps of every set (R380) — Core #18, #41, #60,
    /// #71, #96 and Classic+ #22 Blood Moon, the one new Cost (1) Trap (docs/classic-sets.md B2.6).
    const TRAP_POOL: [&str; 7] =
        ["core-018", "core-041", "core-060", "core-071", "core-096", "classic-010", "classicplus-022"];

    /// TS `catalog.query(args).map((def) => def.id)`.
    fn query_ids(args: Value) -> Vec<String> {
        crate::register_all();
        crate::query::query(&json_as::<CardQuery>(args)).iter().map(|def| def.id.clone()).collect()
    }

    /// The radiant face's pool: every trap of every set, Core #85 (Cost (2)) included.
    fn radiant_trap_pool() -> Vec<String> {
        query_ids(json!({ "type": TRAP_TYPES }))
    }

    /// R82: a turn whose only legal actions are ending it, conceding and offering a draw auto-ends by
    /// itself, and `reduce` runs that check after EVERY action — so a play that empties the hand and
    /// leaves no unit hands the turn over: the opponent draws (taking fatigue on an empty library),
    /// start-of-turn triggers fire, and the numbers under test move underneath the assertion. Every
    /// scenario below therefore keeps one free 0-cost Spell in p1's hand. It is never played; it only
    /// keeps one legal action on the turn. (Reported as a harness gap: `scenario` could hold the turn
    /// open by itself.)
    const ANCHOR: &str = "core-010"; // Rapid Replenish, Spell, cost 0 — always an affordable play.

    /// `scenario(opts)` with ANCHOR appended to p1's hand, the shipped cards registered first.
    fn board(opts: Value) -> Scenario {
        crate::register_all();
        let mut opts = opts;
        let mut p1 = opts.get("p1").cloned().unwrap_or_else(|| json!({}));
        let mut hand = p1.get("hand").and_then(Value::as_array).cloned().unwrap_or_default();
        hand.push(json!(ANCHOR));
        p1["hand"] = Value::Array(hand);
        opts["p1"] = p1;
        scenario(opts)
    }

    const LANE: i32 = 3;

    /// Every backrow zone p1 holds, by catalog id, with `None` for an empty one.
    fn backrow_ids(s: &Scenario) -> Vec<Option<String>> {
        (1..=5).map(|lane| s.backrow(P1, lane).map(|card| card.def_id)).collect()
    }

    /// An engine value as the JSON TS compared (`toEqual` on an object literal).
    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialisable")
    }

    fn in_pool(pool: &[&str], def_id: Option<&str>) -> bool {
        def_id.is_some_and(|id| pool.contains(&id))
    }

    /// TS `toMatchObject`: every key of `pattern` is in `actual` with a matching value.
    fn matches_object(actual: &Value, pattern: &Value) -> bool {
        match (actual, pattern) {
            (Value::Object(have), Value::Object(want)) => want
                .iter()
                .all(|(key, value)| have.get(key).is_some_and(|got| matches_object(got, value))),
            _ => actual == pattern,
        }
    }

    // -------------------------------------------------------------------------------------------
    // The pool (§5.1, R60)
    // -------------------------------------------------------------------------------------------

    #[test]
    fn build_row_67_r380_the_base_pool_is_every_sets_cost_1_traps_and_the_radiant_pool_every_trap() {
        // The base face asks for Cost (1) traps, the radiant face for any trap. Field Trap counts as
        // Trap (§8 #51, R35, R61), and #85 costs 2, so only the radiant query reaches it.
        assert_eq!(query_ids(json!({ "type": TRAP_TYPES, "cost": 1 })), TRAP_POOL);
        let every_trap = query_ids(json!({ "type": TRAP_TYPES }));
        for id in TRAP_POOL.iter().chain(["core-085"].iter()) {
            assert!(every_trap.iter().any(|entry| entry == id), "{id} in {every_trap:?}");
        }
        assert!(every_trap.iter().any(|id| id.starts_with("classic-")));
        assert!(every_trap.iter().all(|id| {
            crate::query::query(&json_as::<CardQuery>(json!({ "defId": id })))
                .first()
                .is_some_and(|def| def.type_.as_str().contains("Trap"))
        }));
    }

    #[test]
    fn r81_the_cry_asks_for_nothing_the_lane_is_the_units_own_not_a_declared_pick() {
        crate::register_all();
        let scripts = super::script();
        assert!(scripts.base.targets.is_empty());
        assert!(scripts.base.modes.is_empty());
        assert!(scripts.radiant.targets.is_empty());
        assert!(scripts.base.cry.is_some());
        assert!(scripts.radiant.cry.is_some());
    }

    // -------------------------------------------------------------------------------------------
    // Base: the summon (§3.1, §3.2, R1, R33)
    // -------------------------------------------------------------------------------------------

    #[test]
    fn s8_3_summons_a_trap_from_the_pool_into_the_units_own_lanes_backrow_zone() {
        let mut s = board(json!({ "p1": { "hand": [OOMEN] } }));
        s.play(OOMEN, json!({ "zone": LANE }));

        let trap = s.backrow(P1, LANE);
        assert!(trap.is_some());
        let def_id = trap.as_ref().map(|card| card.def_id.clone());
        assert!(in_pool(&TRAP_POOL, def_id.as_deref()));
        // §3.1: only that column, never the leftmost free zone (R64's fallback is not what this says).
        assert_eq!(backrow_ids(&s), vec![None, None, def_id, None, None]);
    }

    #[test]
    fn s3_2_r33_the_trap_arrives_face_down_and_not_radiant_and_r1_leaves_it_unpaid() {
        let mut s = board(json!({ "p1": { "hand": [OOMEN] } }));
        s.play(OOMEN, json!({ "zone": LANE }));

        assert_ne!(s.backrow(P1, LANE).and_then(|card| card.face_up), Some(true));
        // The base face makes an ordinary trap: "Radiant" is the radiant face's word (R275).
        assert_eq!(s.backrow(P1, LANE).map(|card| card.radiant), Some(false));
        // Only Oomen's own cost of 1 was paid: a summon pays nothing (R1, §6.3 Summon).
        s.expect_mana(P1, 3).expect_events(json!(["cardPlayed", "summoned", "summoned"]));
    }

    #[test]
    fn r60_the_seeded_pick_stays_inside_the_pool_and_reaches_every_member_of_it() {
        let mut seen: IndexSet<String> = IndexSet::new();
        for seed in 0..40 {
            let mut s = board(json!({ "seed": format!("oomen-{seed}"), "p1": { "hand": [OOMEN] } }));
            s.play(OOMEN, json!({ "zone": LANE }));
            let trap = s.backrow(P1, LANE);
            assert!(trap.is_some());
            assert!(in_pool(&TRAP_POOL, trap.as_ref().map(|card| card.def_id.as_str())));
            if let Some(trap) = trap {
                seen.insert(trap.def_id);
            }
        }
        let mut seen: Vec<String> = seen.into_iter().collect();
        seen.sort();
        let mut pool: Vec<String> = TRAP_POOL.iter().map(|id| id.to_string()).collect();
        pool.sort();
        assert_eq!(seen, pool);
    }

    #[test]
    fn r60_the_same_seed_gives_the_same_trap_the_pick_replays_s9_3() {
        let pick = || -> Option<String> {
            let mut s = board(json!({ "seed": "oomen-replay", "p1": { "hand": [OOMEN] } }));
            s.play(OOMEN, json!({ "zone": LANE }));
            s.backrow(P1, LANE).map(|card| card.def_id)
        };
        assert_eq!(pick(), pick());
    }

    // -------------------------------------------------------------------------------------------
    // Base: the fizzle (R47, §8 Conventions)
    // -------------------------------------------------------------------------------------------

    #[test]
    fn r47_an_occupied_backrow_zone_fizzles_the_summon_and_the_unit_still_enters() {
        let mut s = board(json!({ "p1": { "hand": [OOMEN], "backrow": [{ "def": MANA_WELL, "lane": LANE }] } }));
        s.play(OOMEN, json!({ "zone": LANE }));

        s.expect_in_zone(OOMEN, "field");
        // Nothing moved, nothing spilled into another lane (§3.1: this card names one zone only).
        assert_eq!(backrow_ids(&s), vec![None, None, Some(MANA_WELL.to_string()), None, None]);
    }

    #[test]
    fn r688_a_locked_backrow_zone_takes_the_summon_a_lock_refuses_only_plays_and_this_text_names_none() {
        let mut s = board(json!({ "p1": { "hand": [OOMEN] } }));
        // §3.2 Lock is a zone flag. The harness exposes no way to lock a zone (reported as a harness
        // gap: `SideSetup.locks` or `s.lock(player, row, lane)`), and #36 Magic Jammed only locks the
        // zone of a backrow card it destroys, so the flag is set here directly — a test-only liberty.
        s.state_mut().players.p1.locks.backrow[(LANE - 1) as usize] = true;
        s.play(OOMEN, json!({ "zone": LANE }));

        s.expect_in_zone(OOMEN, "field");
        assert!(in_pool(&TRAP_POOL, s.backrow(P1, LANE).map(|card| card.def_id).as_deref()));
        assert_ne!(s.backrow(P1, LANE).and_then(|card| card.face_up), Some(true));
    }

    #[test]
    fn s3_1_lane_1_and_lane_5_are_read_as_the_units_own_column_too() {
        for lane in [1, 5] {
            let mut s = board(json!({ "p1": { "hand": [OOMEN] } }));
            s.play(OOMEN, json!({ "zone": lane }));
            assert!(s.backrow(P1, lane).is_some());
            assert!(in_pool(&TRAP_POOL, s.backrow(P1, lane).map(|card| card.def_id).as_deref()));
        }
    }

    #[test]
    fn s3_1_the_trap_never_lands_on_the_opponents_side_your_backrow_zone() {
        let mut s = board(json!({ "p1": { "hand": [OOMEN] } }));
        s.play(OOMEN, json!({ "zone": LANE }));
        let theirs: Vec<Option<CardInstance>> = (1..=5).map(|lane| s.backrow(P2, lane)).collect();
        assert_eq!(theirs, vec![None, None, None, None, None]);
    }

    // -------------------------------------------------------------------------------------------
    // Radiant: "a random Radiant Trap" (§8 Conventions, R275)
    // -------------------------------------------------------------------------------------------

    #[test]
    fn r275_the_radiant_face_is_2_4_and_summons_a_radiant_trap_into_its_own_lane_face_down() {
        let radiant_pool = radiant_trap_pool();
        let mut s = board(json!({ "p1": { "hand": [{ "def": OOMEN, "radiant": true }] } }));
        s.play(OOMEN, json!({ "zone": LANE }));

        s.expect_stats(OOMEN, json!({ "attack": 2, "health": 4, "maxHealth": 4 }));
        let trap = s.backrow(P1, LANE);
        assert!(trap.is_some());
        let trap = trap.expect("a trap in lane 3");
        assert!(radiant_pool.contains(&trap.def_id));
        assert!(trap.radiant);
        assert_ne!(trap.face_up, Some(true));
        assert_eq!(backrow_ids(&s), vec![None, None, Some(trap.def_id.clone()), None, None]);
        // Still a summon: only Oomen's own cost was paid (R1).
        s.expect_mana(P1, 3);
    }

    #[test]
    fn r33_r97_the_radiant_trap_is_hidden_from_the_opponent_face_and_all_its_controller_reads_it() {
        let mut s = board(json!({
            "seed": "oomen-radiant-hidden",
            "p1": { "hand": [{ "def": OOMEN, "radiant": true }] }
        }));
        s.play(OOMEN, json!({ "zone": LANE }));
        let Some(trap) = s.backrow(P1, LANE) else {
            panic!("the Radiant Oomen should have summoned a trap");
        };
        let at = (LANE - 1) as usize;

        // The opponent is told the zone is occupied and what its back shows (R351), nothing more (§10.8).
        let theirs = s.view(P2);
        let shown_cost = js(&s.view(P1).you.backrow[at]).get("cost").cloned();
        let mut back = json!({ "faceDown": true });
        if let Some(cost) = shown_cost.filter(|cost| !cost.is_null()) {
            back["cost"] = cost;
        }
        assert_eq!(js(&theirs.opponent.backrow[at]), back);
        // Nowhere in their view — the board, the events, a prompt — is the card named or its face shown.
        let serialized = serde_json::to_string(&theirs).expect("a view serialises");
        assert!(!serialized.contains(&format!("\"{}\"", trap.id)));
        assert!(!serialized.contains(&format!("\"{}\"", trap.def_id)));

        // Its controller reads it, Radiant face included (R33).
        let mine = js(&s.view(P1).you.backrow[at]);
        assert!(matches_object(
            &mine,
            &json!({ "faceDown": false, "defId": trap.def_id, "radiant": true })
        ));
    }

    #[test]
    fn s8_3_the_radiant_pool_drops_the_cost_clause_every_pick_is_a_trap_of_any_set_each_radiant() {
        let radiant_pool = radiant_trap_pool();
        let mut seen: IndexSet<String> = IndexSet::new();
        for seed in 0..40 {
            let mut s = board(json!({
                "seed": format!("oomen-radiant-{seed}"),
                "p1": { "hand": [{ "def": OOMEN, "radiant": true }] }
            }));
            s.play(OOMEN, json!({ "zone": LANE }));
            // The pick is read off its `summoned` event, not off lane 3's zone: R403 makes C #88 Siphon
            // Squad live from the moment it is set, its self-Tribute included, and p2 controls no Units
            // here, so the state check right after the summon Tributes it. It still came, Radiant.
            let picks: Vec<String> = s
                .events()
                .iter()
                .map(js)
                .filter(|event| event["type"] == "summoned" && event["row"] == "backrow")
                .filter_map(|event| event["instanceId"].as_str().map(str::to_string))
                .collect();
            assert_eq!(picks.len(), 1);
            let trap = s.card(picks.first().map(String::as_str).unwrap_or("")).clone();
            assert!(trap.radiant);
            assert!(radiant_pool.contains(&trap.def_id));
            // Every other pick stays where §3.1 put it.
            if trap.def_id == SIPHON_SQUAD {
                s.expect_in_zone(trap.id.as_str(), "graveyard");
            } else {
                assert_eq!(s.backrow(P1, LANE).map(|card| card.id), Some(trap.id.clone()));
            }
            seen.insert(trap.def_id);
        }
        // Forty seeds over a pool of every set's traps reach beyond the Cost (1) ones.
        assert!(seen.iter().any(|id| !TRAP_POOL.contains(&id.as_str())));
        assert!(seen.len() > TRAP_POOL.len());
    }

    #[test]
    fn r47_the_radiant_face_fizzles_on_an_occupied_zone_and_the_unit_still_enters() {
        let mut s = board(json!({
            "p1": { "hand": [{ "def": OOMEN, "radiant": true }], "backrow": [{ "def": MANA_WELL, "lane": LANE }] }
        }));
        s.play(OOMEN, json!({ "zone": LANE }));

        s.expect_in_zone(OOMEN, "field");
        assert_eq!(backrow_ids(&s), vec![None, None, Some(MANA_WELL.to_string()), None, None]);
        // The Field Spell already there is not made Radiant: the summon made nothing.
        assert_eq!(s.backrow(P1, LANE).map(|card| card.radiant), Some(false));
    }

    #[test]
    fn r688_the_radiant_face_summons_into_a_locked_zone_too_and_the_unit_still_enters() {
        let radiant_pool = radiant_trap_pool();
        let mut s = board(json!({ "p1": { "hand": [{ "def": OOMEN, "radiant": true }] } }));
        s.state_mut().players.p1.locks.backrow[(LANE - 1) as usize] = true;
        s.play(OOMEN, json!({ "zone": LANE }));

        s.expect_in_zone(OOMEN, "field").expect_stats(OOMEN, json!({ "attack": 2, "health": 4 }));
        assert!(s.backrow(P1, LANE).is_some_and(|card| radiant_pool.contains(&card.def_id)));
    }
}
