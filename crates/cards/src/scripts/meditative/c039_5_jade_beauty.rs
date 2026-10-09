//! Meditative #39.5 Jade Beauty (SPEC §8.8 row 39.5). (10) Unit, CN, Token (printed Mythic),
//! 20/20 → 40/40.
//!
//!   Base:    "Can't attack, Indestructible, Immutable
//!            End of turn: Allure every enemy Unit. At the start of your next turn they join your
//!            side, or die of heartbreak if you have no room.
//!            You summon this when your Jade Counter reaches 5."
//!   Radiant: "Indestructible, Immutable
//!            End of turn: Allure every enemy Unit. At the start of your next turn they join your
//!            side, or die of heartbreak if you have no room.
//!            Your Jade Beauties become Radiant when your Jade Counter reaches 10."
//!
//! Engine: a Unit token that ME-JADE summons (no Cry). Indestructible (R46). Immutable (R23): no
//! Nerf, Buff, Transform or Fuse-onto, though Radiant still applies. Each end of its controller's
//! turn (R62) it runs ME-ALLURE over every enemy Unit then on the field (MD-C4). The counter's 10
//! turns it Radiant on the field (§5.2): 40/40 with its damage kept, Can't attack gone, nothing
//! fires again. Handed out as a card by C+ #23's every-token pool (R382), it is played at its
//! printed (10), with no Cry to fire.
//! Tunes: none; the thresholds are ME-JADE's config.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-039-5";

fn beauty() -> Script {
    Script {
        end_of_turn: Some(hook(|_ctx| vec![allure_enemy_units()])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // Both faces Allure alike; the Radiant face's stats and keywords are catalog data.
    let base = beauty();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Meditative #39.5 Jade Beauty — SPEC §8.8 row 39.5, BUILD M10 row M 39.5: "Summoned with no Cry
// when its player's Jade Counter crosses 5, once a game, none at a full row with the threshold
// spent (MD-C3); it can't attack; Indestructible and Immutable (no Nerf, Buff or Transform, R23);
// at its controller's end of turn it marks every enemy Unit then on the field (`allure`, public),
// and at their next start of turn each marked Unit still the opponent's is stolen in lane order
// (R15), summoning sick, or destroyed with no open zone, its Death and Reborn firing, an
// Indestructible one staying (MD-C4); a marked Unit that left meanwhile is not taken; the Allure
// lands even after the Beauty is gone (R76); crossing 10 makes every Jade Beauty its player
// controls Radiant, or summons a Radiant one; radiant 40/40, its damage kept, and it may attack".
//
// Every case here crosses a turn boundary, so both sides keep a card in hand, a unit on the board
// and a few library cards: the engine auto-ends a turn with nothing meaningful left (R82), and an
// empty library would add fatigue damage to the assertions.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-008";
    const LIBRARY: [&str; 3] = ["core-025", "core-008", "core-020"];
    /// #17 Flood (Bounce all Units).
    const FLOOD: &str = "core-017";
    /// #34 Collateral Damage (Exile a permanent).
    const EXILE: &str = "core-034";
    /// #66 The Rock (10/10 Indestructible, the wall).
    const WALL: &str = "core-066";
    /// #81 Radiant Saintess (Reborn; Death: all your other units become Radiant).
    const SAINTESS: &str = "core-081";
    /// #3 Right-house Defender (Divine Shield, Reborn).
    const DEFENDER: &str = "core-003";
    /// #16 Hit Job (Destroy target unit).
    const HIT_JOB: &str = "core-016";
    /// C+ #72 Book of Nerf.
    const NERF: &str = "classicplus-072";

    fn beauty_lane(s: &Scenario) -> i32 {
        (1..=5)
            .find(|lane| s.unit(P1, *lane).is_some_and(|unit| unit.def_id == ID))
            .expect("a Jade Beauty on p1's field")
    }

    #[test]
    fn r963_end_of_turn_allures_and_the_next_start_steals_summoning_sick() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "field": [{ "def": ID, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [FILLER], "field": [{ "def": FILLER, "lane": 3 }], "library": LIBRARY },
        }));
        let prey = s.unit(P2, 3).expect("p2's unit").id.clone();

        // Nothing is taken on the turn it is Allured.
        s.end_turn();
        assert_eq!(s.card(&prey).controller, P2);

        // At the Allurer's next start of turn it joins their side, summoning sick.
        s.end_turn();
        let taken = s.card(&prey);
        assert_eq!(taken.controller, P1);
        assert_eq!(taken.summoned_turn, Some(s.state().turn));
        // Lane order, R15: the same lane on the taker's side, which stood open.
        assert_eq!(s.unit(P1, 3).map(|unit| unit.id), Some(prey));
    }

    #[test]
    fn r963_no_room_destroys_core_081_firing_its_death_and_core_003_reborn() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": {
                "hand": [FILLER],
                "field": [
                    { "def": ID, "lane": 1 },
                    { "def": FILLER, "lane": 2 },
                    { "def": FILLER, "lane": 3 },
                    { "def": FILLER, "lane": 4 },
                    { "def": FILLER, "lane": 5 },
                ],
                "library": LIBRARY,
            },
            "p2": {
                "hand": [FILLER, FILLER],
                "field": [
                    { "def": SAINTESS, "radiant": true, "lane": 1 },
                    { "def": DEFENDER, "lane": 2 },
                    { "def": FILLER, "lane": 3 },
                ],
                "library": LIBRARY,
            },
        }));

        s.end_turn();
        s.end_turn();

        // No open zone: every marked Unit is destroyed instead of stolen.
        let destroyed = s
            .events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Destroyed { .. }))
            .count();
        assert!(destroyed >= 3, "all three marked Units are destroyed");
        // The Saintess's Death fired: her Radiant face turns every card in p2's hand Radiant.
        assert!(
            s.hand(P2).iter().all(|card| card.radiant),
            "the Saintess's Death fired"
        );
        // The Defender's Reborn brought it back at 1 health.
        let reborn = s.unit(P2, 2).expect("the Defender's Reborn body");
        assert_eq!(reborn.def_id, DEFENDER);
        s.expect_stats(&reborn.id, json!({ "attack": 1, "health": 1 }));
        // The Beauty and its side stand untouched.
        assert_eq!(s.unit(P1, beauty_lane(&s)).map(|unit| unit.def_id), Some(ID.to_string()));
    }

    #[test]
    fn r963_core_066_stays() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": {
                "hand": [FILLER],
                "field": [
                    { "def": ID, "lane": 1 },
                    { "def": FILLER, "lane": 2 },
                    { "def": FILLER, "lane": 3 },
                    { "def": FILLER, "lane": 4 },
                    { "def": FILLER, "lane": 5 },
                ],
                "library": LIBRARY,
            },
            "p2": { "hand": [FILLER], "field": [{ "def": WALL, "lane": 2 }], "library": LIBRARY },
        }));
        let wall = s.unit(P2, 2).expect("the wall").id.clone();

        s.end_turn();
        s.end_turn();

        // R46: an Indestructible Unit stays where it is, under its own side.
        let kept = s.card(&wall);
        assert_eq!(kept.controller, P2);
        assert_eq!(s.unit(P2, 2).map(|unit| unit.id), Some(wall));
    }

    #[test]
    fn r963_a_unit_bounced_meanwhile_is_not_taken() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "field": [{ "def": ID, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [FILLER, FLOOD], "field": [{ "def": FILLER, "lane": 3 }], "library": LIBRARY },
        }));
        let prey = s.unit(P2, 3).expect("p2's unit").id.clone();

        s.end_turn();
        // Bounced meanwhile (R174): the mark goes with the stay, so the next start takes nothing.
        // Flood bounces every Unit; the token Beauty vanishes (R11), the prey returns to its hand.
        s.play(FLOOD, json!({}));
        s.end_turn();

        let kept = s.card(&prey);
        assert_eq!(kept.controller, P2);
        assert!(s.hand(P2).iter().any(|card| card.id == prey));
    }

    #[test]
    fn r963_lands_after_the_beauty_is_gone() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "field": [{ "def": ID, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [EXILE, FILLER], "field": [{ "def": FILLER, "lane": 3 }], "library": LIBRARY },
        }));
        let prey = s.unit(P2, 3).expect("p2's unit").id.clone();
        let lane = beauty_lane(&s);
        let beauty = s.unit(P1, lane).expect("the Beauty").id.clone();

        s.end_turn();
        // R76: the Beauty is exiled (Immutable stops Nerf, Buff, Transform and Fuse — not exile),
        // yet the Allure still lands at p1's next start.
        s.play(EXILE, json!({ "targets": [{ "pick": "instance", "instanceId": beauty }] }));
        assert!(s.unit(P1, lane).is_none());
        s.end_turn();

        assert_eq!(s.card(&prey).controller, P1);
    }

    #[test]
    fn cant_attack_and_core_016_hit_job_leaves_it() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "field": [{ "def": ID, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [HIT_JOB, FILLER], "field": [{ "def": FILLER, "lane": 3 }], "library": LIBRARY },
        }));
        let lane = beauty_lane(&s);
        let beauty = s.unit(P1, lane).expect("the Beauty").id.clone();

        // Can't attack: its attack is refused.
        s.expect_refused(|s| s.attack(&beauty, "hero"));

        // Hit Job marks it for destroy, but Indestructible (R46) leaves it standing.
        s.end_turn();
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": beauty }] }));
        s.end_turn();
        assert_eq!(s.unit(P1, lane).map(|unit| unit.id), Some(beauty));
    }

    #[test]
    fn r23_classicplus_072_book_of_nerf_changes_nothing() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "field": [{ "def": ID, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [NERF, FILLER], "field": [{ "def": FILLER, "lane": 3 }], "library": LIBRARY },
        }));
        let lane = beauty_lane(&s);
        let beauty = s.unit(P1, lane).expect("the Beauty").id.clone();

        s.end_turn();
        s.play(NERF, json!({ "targets": [{ "pick": "instance", "instanceId": beauty }] }));

        // R23 Immutable: no Nerf lands — still 20/20.
        s.expect_stats(&beauty, json!({ "attack": 20, "health": 20 }));
    }

    #[test]
    fn radiant_40_40_may_attack_damage_kept() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(
            [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
            [Some(20), Some(20), Some(40), Some(40)]
        );
        // The Radiant face lost Can't attack: it may attack.
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "field": [{ "def": ID, "radiant": true, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [FILLER], "field": [{ "def": FILLER, "lane": 3 }], "library": LIBRARY },
        }));
        let lane = beauty_lane(&s);
        let beauty = s.unit(P1, lane).expect("the Radiant Beauty").id.clone();
        let prey = s.unit(P2, 3).expect("p2's unit").id.clone();
        s.attack(&beauty, &prey);
        assert!(s.card(&prey).zone.z() != ZoneName::Field, "the 40 hit kills the prey");

        // §5.2: made Radiant on the field, its damage is kept — a Dud-marked Beauty ascends hurt.
        let mut s = scenario(json!({
            "p1": { "hand": ["meditative-039-4", "meditative-039-4", FILLER], "field": [{ "def": ID, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": ["meditative-039-3", FILLER], "library": LIBRARY },
        }));
        let lane = beauty_lane(&s);
        let scarred = s.unit(P1, lane).expect("the Beauty").id.clone();
        s.play("meditative-039-4", json!({}));
        s.end_turn();
        s.play("meditative-039-3", json!({ "targets": [{ "pick": "instance", "instanceId": scarred }] }));
        s.end_turn();
        s.play("meditative-039-4", json!({}));
        let ascended = s.card(&scarred);
        assert!(ascended.radiant, "crossing 10 made it Radiant");
        s.expect_stats(&scarred, json!({ "attack": 40, "health": 38 }));
    }
}
