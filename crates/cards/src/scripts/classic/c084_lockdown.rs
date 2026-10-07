//! C #84 Lockdown (SPEC §8.6 row 84). (2) Field Spell, Rare.
//!   Base:    "Indestructible\nAfter a permanent is played, Lock its zone.\nActivate: Tribute this."
//!   Radiant: "Indestructible\nAfter your opponent plays a permanent, Lock its zone.\nActivate: Tribute this."
//!   Engine:  "A trigger on `cardPlayed` of a permanent (either player's; Radiant: the opponent's) that
//!            Locks (Lock / Unlock, §6.3) the zone the card entered; the card stays, since Lock evicts
//!            nothing. The designer's "cast" is "played": a cast by an effect counts (R70); a summon
//!            that is no play (a token, a Recruit) does not; Lockdown doesn't answer its own arrival
//!            (R119). Activate (§6.2, R384), once per turn: Tribute this, which bypasses Indestructible
//!            (§6.3 Sacrifice). Tunes: none."
//!
//! The trigger answers `cardPlayed` — a play or a cast (R70), never a summon, a Recruit, a Reborn or a
//! countered play, none of which emits it — and `lockPlayedZone` Locks the zone the played card stands
//! in, the card staying (§3.2); a played Spell stands in none and locks nothing. R119: a permanent that
//! is not a trap gets no filter for its own arrival, so the trigger passes over the `cardPlayed` naming
//! Lockdown itself. The `locked` event names the zone only, so a face-down Trap's zone locks without the
//! other player learning the Trap (R33). The Radiant face answers only the opponent's plays.
//!
//! Indestructible is the catalog keyword: a destroy leaves it (R46); an exile removes it. The Activate is
//! R384's once-per-turn ability whose cost is "Tribute this" (`tributeSelf`): a Sacrifice, which
//! bypasses Indestructible (§6.3), and which is the whole of the ability. The Locks it made stay after it
//! leaves.
//!
//! Rulings: R33, R46, R70, R119, R384. Its proof: `test/classic/084-lockdown.test.ts`.

use jackioh_engine::effects::lock_played_zone;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-084";

fn lock_after_play(opponent_only: bool) -> TriggerDef {
    TriggerDef::new("lock-played-zone", &[GameEventType::CardPlayed], move |ctx, event| {
        let GameEvent::CardPlayed { player, instance_id, .. } = event else {
            return vec![];
        };
        // R119: not its own arrival.
        if ctx.self_.as_ref().is_some_and(|me| me.id == *instance_id) {
            return vec![];
        }
        if opponent_only && *player == ctx.controller {
            return vec![];
        }
        vec![lock_played_zone(json_as(json!({ "event": event })))]
    })
}

fn lockdown(opponent_only: bool) -> Script {
    Script {
        triggers: vec![lock_after_play(opponent_only)],
        activations: vec![ActivationDecl {
            id: "tribute".into(),
            label: "Tribute this".into(),
            uses: ActivationUses::Count(1),
            cost: Some(ActivationCost {
                tribute_self: Some(true),
                ..ActivationCost::default()
            }),
            targets: vec![],
            modes: vec![],
            can_activate: None,
            has: None,
            run: hook(|_ctx| vec![]),
        }],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: lockdown(false),
        radiant: lockdown(true),
    }
}

// C #84 Lockdown (SPEC §8.6 row 84; BUILD M9 row C 84). (2) Field Spell, Rare: Indestructible; after
// a permanent is played, Lock its zone; Activate: Tribute this. Radiant: after your opponent plays a
// permanent.
//
// A countered card is shown with the real C #72 Grand Counterspell. No card in the catalog casts a
// permanent by its effect (C #7, #47 and #56 cast Spells only; no Cast-on-draw card with a script is a
// permanent), so "a cast counts" uses a fixture Spell that casts a Mr. Vanilla (`castNew`, R70), as
// `test/combat-windows.test.ts` does.
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::{CastNewArgs, cast_new};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const LOCKDOWN: &str = "classic-084";
    const VANILLA: &str = "core-008"; // 4/4
    const TOKEN_MAKER: &str = "core-015"; // Me and Mr Token: Cry: summon a Rush Token
    const DEFENDER: &str = "core-003"; // Right-house defender: Reborn
    const CALL_TO_ARMS: &str = "core-069"; // Spell 2: Recruit 3 (1) Cost or less Units
    const HIT_JOB: &str = "core-016";
    const MAGIC_JAMMED: &str = "core-036"; // destroy target backrow card, Lock its zone
    const COLLATERAL: &str = "core-034"; // exile target permanent
    const MANA_WELL: &str = "core-006"; // Field Spell
    const SHEEPISH: &str = "core-041"; // Trap
    const STOCKPILE: &str = "core-005";
    const TIMMY: &str = "core-011";
    const GRAND_COUNTERSPELL: &str = "classic-072"; // Trap: when your opponent plays a non-Unit card, Counter it.

    /// The harness, after the catalog and every card script are registered (TS's harness did it on import).
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialisable")
    }

    fn must<T>(value: Option<T>, what: &str) -> T {
        match value {
            Some(found) => found,
            None => panic!("missing: {what}"),
        }
    }

    /// TS `const SPARE: SideSetup = { hand: [STOCKPILE], library: [VANILLA, VANILLA, VANILLA] }`.
    fn spare() -> Value {
        json!({ "hand": [STOCKPILE], "library": spare_library() })
    }

    fn spare_library() -> Value {
        json!([VANILLA, VANILLA, VANILLA])
    }

    /// TS `{ ...side, ...SPARE }`: the side's keys, then SPARE's over them.
    fn with_spare(side: Value) -> Value {
        let mut merged = side;
        if let (Some(into), Value::Object(extra)) = (merged.as_object_mut(), spare()) {
            for (key, value) in extra {
                into.insert(key, value);
            }
        }
        merged
    }

    /// TS `{ ...SPARE, ...side }`: SPARE's keys, then the side's over them.
    fn spare_with(side: Value) -> Value {
        let mut merged = spare();
        if let (Some(into), Value::Object(extra)) = (merged.as_object_mut(), side) {
            for (key, value) in extra {
                into.insert(key, value);
            }
        }
        merged
    }

    fn locked(s: &Scenario, player: PlayerId, row: Row, lane: i32) -> bool {
        is_locked(s.state(), &ZoneSlot { player, row, lane })
    }

    fn lock_events(s: &Scenario) -> Vec<String> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Locked { player, row, lane } => Some(format!("{player} {row} {lane}")),
                _ => None,
            })
            .collect()
    }

    /// A fixture card: a transient def in the match state and its script in the registry.
    fn fixture(s: &mut Scenario, id: &str, type_: CardType, script: Script) -> String {
        let face = json!({ "keywords": [], "text": id });
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
            "base": face,
            "radiant": face
        }));
        s.state_mut().transient_defs.insert(id.to_string(), def);
        let mut scripts = registered_scripts();
        scripts.insert(
            id.to_string(),
            CardScripts {
                base: script.clone(),
                radiant: script,
            },
        );
        register_scripts(scripts);
        id.to_string()
    }

    /// base
    mod base {
        use super::*;

        /// §8.6 after a Unit is played its zone Locks, and the Unit stays
        #[test]
        fn s8_6_after_a_unit_is_played_its_zone_locks_and_the_unit_stays() {
            let mut s = scenario(json!({
                "p1": { "backrow": [LOCKDOWN], "hand": [VANILLA, STOCKPILE], "library": spare_library() },
                "p2": spare()
            }));
            s.play(VANILLA, json!({ "zone": 3 }));
            assert_eq!(lock_events(&s), ["p1 units 3"]);
            assert_eq!(s.unit(P1, 3).map(|card| card.def_id), Some(VANILLA.to_string()));
        }

        /// §8.6 a Field Spell's zone and a face-down Trap's zone lock too, the other seat never told which Trap (R33)
        #[test]
        fn s8_6_a_field_spell_s_zone_and_a_face_down_trap_s_zone_lock_too_the_other_seat_never_told_which_trap_r33() {
            let mut s = scenario(json!({
                "p1": { "backrow": [LOCKDOWN], "hand": [MANA_WELL, SHEEPISH, STOCKPILE], "library": spare_library(), "mana": 10 },
                "p2": spare()
            }));
            s.play(MANA_WELL, json!({ "zone": 2 }));
            assert_eq!(lock_events(&s), ["p1 backrow 2"]);
            s.play(SHEEPISH, json!({ "zone": 3 }));
            assert_eq!(lock_events(&s), ["p1 backrow 3"]);
            assert_eq!(s.backrow(P1, 3).map(|card| card.def_id), Some(SHEEPISH.to_string()));
            let seen = js(&s.view(P2))["events"].clone();
            let locks: Vec<Value> = seen
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|event| event["type"] == json!("locked"))
                .collect();
            assert!(locks.contains(&json!({ "type": "locked", "player": "p1", "row": "backrow", "lane": 3 })));
            // No event p2 is shown names the Trap.
            let trap = must(s.backrow(P1, 3), "p1's Trap");
            let text = serde_json::to_string(&seen).expect("serialisable");
            assert!(!text.contains(SHEEPISH));
            assert!(!text.contains(&format!("\"{}\"", trap.id)));
        }

        /// §8.6 either player's plays: the opponent's Unit Locks its zone on their turn
        #[test]
        fn s8_6_either_player_s_plays_the_opponent_s_unit_locks_its_zone_on_their_turn() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": with_spare(json!({ "backrow": [LOCKDOWN] })),
                "p2": { "hand": [VANILLA, STOCKPILE], "library": spare_library() }
            }));
            s.play(VANILLA, json!({ "zone": 4 }));
            assert!(locked(&s, P2, Row::Units, 4));
        }

        /// §8.6 a Spell stands in no zone and locks nothing
        #[test]
        fn s8_6_a_spell_stands_in_no_zone_and_locks_nothing() {
            let mut s = scenario(json!({
                "p1": { "backrow": [LOCKDOWN], "hand": [STOCKPILE, TIMMY], "library": spare_library() },
                "p2": spare()
            }));
            s.play(STOCKPILE, json!({}));
            assert!(lock_events(&s).is_empty());
        }

        /// R119 Lockdown does not answer its own arrival
        #[test]
        fn r119_lockdown_does_not_answer_its_own_arrival() {
            let mut s = scenario(json!({ "p1": { "hand": [LOCKDOWN, STOCKPILE], "library": spare_library() }, "p2": spare() }));
            s.play(LOCKDOWN, json!({ "zone": 1 }));
            assert!(lock_events(&s).is_empty());
            assert!(!locked(&s, P1, Row::Backrow, 1));
        }

        /// R70 a summon that is no play locks nothing: a Cry's token, a Recruit, a Reborn
        #[test]
        fn r70_a_summon_that_is_no_play_locks_nothing_a_cry_s_token_a_recruit_a_reborn() {
            let mut s = scenario(json!({
                "p1": {
                    "backrow": [LOCKDOWN],
                    "field": [{ "def": DEFENDER, "lane": 5 }],
                    "hand": [TOKEN_MAKER, CALL_TO_ARMS, HIT_JOB, STOCKPILE],
                    "library": [TIMMY, VANILLA],
                    "mana": 20
                },
                "p2": spare()
            }));
            // Me and Mr Token is played (lane 1, Locked); the Rush Token it summons (lane 2) is not.
            s.play(TOKEN_MAKER, json!({ "zone": 1 }));
            assert_eq!(lock_events(&s), ["p1 units 1"]);
            assert_eq!(s.unit(P1, 2).map(|card| card.def_id), Some("core-t-rush".to_string()));
            // Call to Arms recruits Timmy into lane 3: no play, no Lock.
            s.play(CALL_TO_ARMS, json!({}));
            assert_eq!(s.unit(P1, 3).map(|card| card.def_id), Some(TIMMY.to_string()));
            assert!(lock_events(&s).is_empty());
            // The Defender dies and comes back through Reborn: no play, no Lock.
            let defender = must(s.unit(P1, 5), "the Defender");
            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": defender.id }] }));
            assert_eq!(s.unit(P1, 5).map(|card| card.id), Some(defender.id.clone()));
            assert!(!locked(&s, P1, Row::Units, 5));
        }

        /// R70 a cast by an effect counts as a play: the cast Unit's zone Locks, the casting Spell's nothing
        #[test]
        fn r70_a_cast_by_an_effect_counts_as_a_play_the_cast_unit_s_zone_locks_the_casting_spell_s_nothing() {
            let mut s = scenario(json!({
                "p1": { "backrow": [LOCKDOWN], "hand": [STOCKPILE, TIMMY], "library": spare_library() },
                "p2": spare()
            }));
            let caster = fixture(
                &mut s,
                "fx-cast-vanilla",
                CardType::Spell,
                Script {
                    cry: Some(hook(|_ctx| {
                        vec![cast_new(CastNewArgs {
                            def: VANILLA.into(),
                            radiant: None,
                            how: Default::default(),
                        })]
                    })),
                    ..Script::default()
                },
            );
            let spell = new_instance(s.state_mut(), &caster, P1, Zone::Hand { player: P1 });
            s.state_mut().players.p1.hand.push(spell.clone());
            s.play(&spell, json!({}));
            let cast = must(s.unit(P1, 1), "the cast Vanilla");
            assert_eq!(cast.def_id, VANILLA);
            assert!(s.last_events().iter().any(
                |event| matches!(event, GameEvent::CardPlayed { instance_id, .. } if *instance_id == cast.id)
            ));
            assert_eq!(lock_events(&s), ["p1 units 1"]);
        }

        /// §8.6 a countered card locks nothing: C #72 Grand Counterspell stops a Field Spell before it reaches a zone
        #[test]
        fn s8_6_a_countered_card_locks_nothing_c_n72_grand_counterspell_stops_a_field_spell_before_it_reaches_a_zone() {
            let mut s = scenario(json!({
                "p1": { "backrow": [LOCKDOWN], "hand": [MANA_WELL, STOCKPILE], "library": spare_library() },
                "p2": spare_with(json!({ "backrow": [{ "def": GRAND_COUNTERSPELL, "faceUp": false }] }))
            }));
            s.play(MANA_WELL, json!({ "zone": 3 }));
            assert!(s.last_events().iter().any(|event| matches!(event, GameEvent::Countered { .. })));
            s.expect_in_zone(MANA_WELL, "graveyard");
            assert!(lock_events(&s).is_empty());
            assert!(!locked(&s, P1, Row::Backrow, 3));
        }

        /// R13 a Lockdown dormant under a backrow pile does not act: a play locks nothing
        #[test]
        fn r13_a_lockdown_dormant_under_a_backrow_pile_does_not_act_a_play_locks_nothing() {
            let mut s = scenario(json!({
                "p1": {
                    "backrow": [LOCKDOWN, { "def": MANA_WELL, "stack": true }],
                    "hand": [VANILLA, STOCKPILE],
                    "library": spare_library()
                },
                "p2": spare()
            }));
            s.play(VANILLA, json!({ "zone": 2 }));
            assert!(lock_events(&s).is_empty());
        }

        /// R46 Indestructible: a destroy leaves it; an exile removes it
        #[test]
        fn r46_indestructible_a_destroy_leaves_it_an_exile_removes_it() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": with_spare(json!({ "backrow": [LOCKDOWN] })),
                "p2": { "hand": [MAGIC_JAMMED, COLLATERAL, STOCKPILE], "library": spare_library(), "mana": 10 }
            }));
            let lockdown = s.card(LOCKDOWN).id.clone();
            s.play(MAGIC_JAMMED, json!({ "targets": [{ "pick": "instance", "instanceId": lockdown }] }));
            s.expect_in_zone(LOCKDOWN, "field");
            let lockdown = s.card(LOCKDOWN).id.clone();
            s.play(COLLATERAL, json!({ "targets": [{ "pick": "instance", "instanceId": lockdown }] }));
            s.expect_in_zone(LOCKDOWN, "exile");
        }

        /// R384 Activate: Tribute this, which bypasses Indestructible; the Locks it made stay
        #[test]
        fn r384_activate_tribute_this_which_bypasses_indestructible_the_locks_it_made_stay() {
            let mut s = scenario(json!({
                "p1": { "backrow": [LOCKDOWN], "hand": [VANILLA, STOCKPILE], "library": spare_library() },
                "p2": spare()
            }));
            s.play(VANILLA, json!({ "zone": 2 }));
            let lockdown = s.card(LOCKDOWN).clone();
            assert!(
                legal_actions(s.state(), P1)
                    .iter()
                    .map(|action| js(action))
                    .any(|action| action["type"] == json!("activate") && action["instanceId"] == json!(lockdown.id))
            );
            assert!(!legal_actions(s.state(), P2).iter().any(|action| js(action)["type"] == json!("activate")));
            s.activate(LOCKDOWN, json!({ "ability": "tribute" }));
            s.expect_in_zone(&lockdown, "graveyard");
            assert!(locked(&s, P1, Row::Units, 2));
        }
    }

    /// radiant
    mod radiant {
        use super::*;

        /// §8.6 only the opponent's plays: your own Unit locks nothing, theirs locks its zone
        #[test]
        fn s8_6_only_the_opponent_s_plays_your_own_unit_locks_nothing_theirs_locks_its_zone() {
            let mut s = scenario(json!({
                "p1": { "backrow": [{ "def": LOCKDOWN, "radiant": true }], "hand": [VANILLA, STOCKPILE], "library": spare_library() },
                "p2": { "hand": [VANILLA, STOCKPILE], "library": spare_library() }
            }));
            s.play(VANILLA, json!({ "zone": 1 }));
            assert!(lock_events(&s).is_empty());
            s.end_turn();
            s.play(VANILLA, json!({ "zone": 1 }));
            assert!(locked(&s, P2, Row::Units, 1));
            assert!(!locked(&s, P1, Row::Units, 1));
        }

        /// R384 the Radiant face keeps Indestructible and its Activate
        #[test]
        fn r384_the_radiant_face_keeps_indestructible_and_its_activate() {
            let mut s = scenario(json!({
                "p1": { "backrow": [{ "def": LOCKDOWN, "radiant": true }], "hand": [MAGIC_JAMMED, STOCKPILE], "library": spare_library() },
                "p2": spare()
            }));
            let lockdown = s.card(LOCKDOWN).id.clone();
            s.play(MAGIC_JAMMED, json!({ "targets": [{ "pick": "instance", "instanceId": lockdown }] }));
            s.expect_in_zone(LOCKDOWN, "field");
            s.activate(LOCKDOWN, json!({ "ability": "tribute" }));
            s.expect_in_zone(LOCKDOWN, "graveyard");
        }
    }
}
