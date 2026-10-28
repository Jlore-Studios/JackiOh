//! TG #35 Corpse Recycler (SPEC §8.11 row 35). (3) Field Spell, Epic. "After a non-Token Unit dies,
//! summon a Rush Token for its controller." (Radiant: a Radiant Rush Token.)
//! A trigger on `destroyed`, as C+ #19.5 Bot Loser's: a Field Spell's trigger, so both players' Units
//! answer it (R2199006). The event names who controlled the Unit as it died (R172), and the token goes
//! to that player, in R64's leftmost open zone (the event carries no lane, R89); a full row summons
//! nothing (§3.2). A Token's death gives nothing, so the Rush Token it makes never feeds it (§7).

use jackioh_engine::catalog::def_of;
use jackioh_engine::effects::summon;
use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-035";

/// §7: the Rush Token, 3/3 Rush (Radiant 6/6 Rush, Cleave).
const RUSH_TOKEN: &str = "core-t-rush";

/// A non-Token Unit's death (R2199006): the player who controlled it as it died (R172), or nothing.
fn died_unit(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<PlayerId> {
    let GameEvent::Destroyed { def_id, controller, .. } = event else {
        return None;
    };
    let def = def_of(Some(&*ctx.state), def_id);
    (def.type_ == CardType::Unit && !def.is_token()).then_some(*controller)
}

/// The condition is written twice, as Core #18's: a queued trigger takes an empty list as "not met".
fn recycler(radiant: bool) -> Script {
    let trigger = TriggerDef::new("corpse-recycler", &[GameEventType::Destroyed], move |ctx, event| {
        let Some(controller) = died_unit(ctx, event) else {
            return vec![];
        };
        let side = if controller == ctx.controller { "self" } else { "enemy" };
        vec![summon(json_as(json!({ "defId": RUSH_TOKEN, "player": side, "radiant": radiant })))]
    })
    .with_when(|ctx, event| died_unit(ctx, event).is_some());
    Script {
        triggers: vec![trigger],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: recycler(false),
        radiant: recycler(true),
    }
}

// TG #35 Corpse Recycler — SPEC §8.11 row 35, BUILD M13 row TG 35. p1 holds it face-up in backrow
// lane 1 and is active.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const RECYCLER: &str = super::ID;
    const RUSH: &str = "core-t-rush";
    const VANILLA: &str = "core-008"; // (1) 4/4
    const BIGOT: &str = "core-002"; // (2) 4/1
    const TIMMY: &str = "core-011"; // (1) 3/3 Rush, First Strike
    const BIG_FELINOR: &str = "core-043"; // (4) 3/10, Cry: destroy all non-Felinor Units.
    const COLLATERAL: &str = "core-034"; // exiles a permanent
    const HIT_JOB: &str = "core-016";

    fn board(radiant: bool, p1: Value, p2: Value) -> Scenario {
        crate::register_all();
        let mut p1 = p1;
        p1["backrow"] = json!([{ "def": RECYCLER, "radiant": radiant, "faceUp": true, "lane": 1 }]);
        if p1.get("mana").is_none() {
            p1["mana"] = json!(20);
        }
        scenario(json!({ "seed": "corpse-recycler", "p1": p1, "p2": p2 }))
    }

    fn units(s: &Scenario, player: PlayerId) -> Vec<Option<String>> {
        (1..=5).map(|lane| s.unit(player, lane).map(|card| card.def_id)).collect()
    }

    fn tokens(s: &Scenario, player: PlayerId) -> usize {
        units(s, player).iter().filter(|def| def.as_deref() == Some(RUSH)).count()
    }

    mod base {
        use super::*;

        #[test]
        fn r2199006_your_non_token_unit_dies_and_you_get_a_rush_token() {
            let mut s = board(false, json!({ "field": [{ "def": BIGOT, "lane": 3 }] }), json!({ "field": [VANILLA] }));
            let bigot = s.unit(P1, 3).unwrap();
            let vanilla = s.unit(P2, 1).unwrap();
            s.attack(&bigot, &vanilla);

            // R64: the leftmost open zone, not the zone it died in.
            assert_eq!(units(&s, P1)[0].as_deref(), Some(RUSH));
            assert!(s.unit(P1, 3).is_none());
            s.expect_stats(RUSH, json!({ "attack": 3, "health": 3 }));
        }

        #[test]
        fn r172_an_enemy_unit_dies_and_the_enemy_gets_one() {
            // Timmy's First Strike kills the Bigot before it strikes back.
            let mut s = board(false, json!({ "field": [TIMMY] }), json!({ "field": [BIGOT] }));
            let vanilla = s.unit(P1, 1).unwrap();
            let bigot = s.unit(P2, 1).unwrap();
            s.attack(&vanilla, &bigot);

            assert_eq!(tokens(&s, P2), 1);
            assert_eq!(tokens(&s, P1), 0);
        }

        #[test]
        fn r2199006_a_token_dying_gives_nothing() {
            let mut s = board(false, json!({ "field": [VANILLA] }), json!({ "field": [{ "def": RUSH, "statsOverride": { "attack": 1, "health": 1 } }] }));
            let vanilla = s.unit(P1, 1).unwrap();
            let token = s.unit(P2, 1).unwrap();
            s.attack(&vanilla, &token);

            assert_eq!(tokens(&s, P2), 0);
            assert!(!s.events().iter().any(|e| matches!(e, GameEvent::Summoned { .. })));
        }

        #[test]
        fn r64_a_full_row_summons_nothing() {
            // The Bigot tops a Stack pile in lane 1 (§3.2): as it dies the Vanilla beneath resumes there,
            // so the row is still full when the token would come.
            let mut s = board(
                false,
                json!({ "field": [VANILLA, { "def": BIGOT, "stack": true }, VANILLA, VANILLA, VANILLA, VANILLA] }),
                json!({ "field": [VANILLA] }),
            );
            let bigot = s.unit(P1, 1).unwrap();
            assert_eq!(bigot.def_id, BIGOT);
            let target = s.unit(P2, 1).unwrap();
            s.attack(&bigot, &target);

            s.expect_in_zone(BIGOT, "graveyard");
            assert_eq!(tokens(&s, P1), 0);
            assert!(units(&s, P1).iter().all(|def| def.as_deref() == Some(VANILLA)));
        }

        #[test]
        fn r68_several_deaths_in_one_check_give_one_token_each() {
            let mut s = board(
                false,
                json!({ "hand": [BIG_FELINOR, HIT_JOB], "field": [{ "def": VANILLA, "lane": 4 }] }),
                json!({ "field": [VANILLA, TIMMY] }),
            );
            s.play(BIG_FELINOR, json!({ "zone": 1 }));

            // Three non-Felinor Units died: one token for p1 (its Vanilla), two for p2.
            assert_eq!(tokens(&s, P1), 1);
            assert_eq!(tokens(&s, P2), 2);
        }

        #[test]
        fn r153_once_it_is_gone_nothing_happens() {
            let mut s = board(false, json!({ "hand": [COLLATERAL, HIT_JOB], "field": [BIGOT] }), json!({ "field": [VANILLA] }));
            let recycler = s.card(RECYCLER).clone();
            s.play(COLLATERAL, json!({ "targets": [{ "pick": "instance", "instanceId": recycler.id }] }));
            let bigot = s.unit(P1, 1).unwrap();
            let vanilla = s.unit(P2, 1).unwrap();
            s.attack(&bigot, &vanilla);

            assert_eq!(tokens(&s, P1), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r74_a_radiant_rush_token_6_6_rush_cleave() {
            let mut s = board(true, json!({ "field": [BIGOT] }), json!({ "field": [VANILLA] }));
            let bigot = s.unit(P1, 1).unwrap();
            let vanilla = s.unit(P2, 1).unwrap();
            s.attack(&bigot, &vanilla);

            let token = s.unit(P1, 1).expect("a token");
            assert_eq!(token.def_id, RUSH);
            assert!(token.radiant);
            s.expect_stats(RUSH, json!({ "attack": 6, "health": 6 }));
        }
    }
}
