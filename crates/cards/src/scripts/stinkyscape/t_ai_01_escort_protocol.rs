//! S T-AI-1 Escort Protocol (SPEC §7, §8.10 row T-AI-1; R1, R64, R482): (1) Unit, AI, Token (no printed
//! rarity), 1/3 → 2/6, Flying, an AI generated token of the held Stinkyscape set (R1420), built by S21.
//!   Base:    "Flying\nCry: Summon a Drone Interceptor."
//!   Radiant: "Flying\nCry: Summon a Radiant Drone Interceptor."
//! Engine: Flying is catalog data. Cry (R1): a summon of the Drone Interceptor into its controller's
//!   leftmost open unit zone (R64), no Cry; with no open zone nothing. The Interceptor draws its own
//!   combat keyword as it arrives (R781004). Radiant: a Radiant Interceptor. Declares no numbers (R482).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-ai-01";

/// The Drone Interceptor token (§8.10 T-drone-interceptor).
const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![summon(json_as(json!({ "defId": INTERCEPTOR, "radiant": radiant })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S T-AI-1 Escort Protocol — SPEC §8.10 row T-AI-1, BUILD M12 row S T-AI-1: "a 1/3 Flying whose Cry summons
// a Drone Interceptor into the leftmost open unit zone; a full unit row summons nothing; radiant 2/6 and a
// Radiant Interceptor".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const ESCORT: &str = "stinkyscape-t-ai-01";
    const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005";

    /// p1 plays Escort Protocol into unit lane `lane` beside `field`.
    fn played(radiant: bool, field: Value, lane: i32) -> Scenario {
        let mut s = scenario(json!({
            "seed": "escort-protocol",
            "p1": { "hand": [{ "def": ESCORT, "radiant": radiant }, FILLER], "field": field, "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(ESCORT, json!({ "zone": lane }));
        s
    }

    #[test]
    fn r64_its_cry_summons_an_interceptor_into_the_leftmost_open_zone() {
        let s = played(false, json!([{ "def": VANILLA, "lane": 1 }]), 3);
        let escort = s.unit(P1, 3).expect("the Escort in lane 3");
        assert_eq!(escort.def_id, ESCORT);
        assert!(keywords_of(s.state(), s.card(escort.id.as_str())).contains(&Keyword::Flying));
        let drone = s.unit(P1, 2).expect("the Interceptor in lane 2");
        assert_eq!(drone.def_id, INTERCEPTOR);
        assert!(!drone.radiant);
        s.expect_stats(ESCORT, json!({ "attack": 1, "health": 3 }));
    }

    #[test]
    fn r64_with_no_open_zone_after_it_lands_nothing_is_summoned() {
        let s = played(false, json!(vec![VANILLA; 4]), 5);
        assert!(!s.last_events().iter().any(|event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == INTERCEPTOR)));
    }

    #[test]
    fn radiant_2_6_and_a_radiant_interceptor() {
        let s = played(true, json!([]), 1);
        s.expect_stats(ESCORT, json!({ "attack": 2, "health": 6 }));
        let drone = s.unit(P1, 2).expect("the Interceptor in lane 2");
        assert_eq!(drone.def_id, INTERCEPTOR);
        assert!(drone.radiant);
    }
}
