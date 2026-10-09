//! M #97.9 Jlockheed's Headquarters (SPEC §8.8 row 97.9, R1261). (4) Unit, Jlockeed, Token (printed
//! Mythic), 0/20 → 0/50.
//!   Base:    "Tribute 5, Indestructible, Can't attack\nActivate: Fill your board with random Jlockheed Units."
//!   Radiant: "Tribute 5, Indestructible, Can't attack\nActivate 2: Fill your board with random Radiant Jlockheed Units."
//!   Engine:  "Activate (R384; Activate 2 on the Radiant face). C+ #2 Groom Shroom's fill: each empty,
//!            unlocked, unreserved unit zone, left to right (R64), gets a random non-token Jlockeed Unit of
//!            every set (R380; this token is out, §5.1, R387), repeats allowed (R60), summoned, so with no
//!            Cry (R1); the Radiant face's second activation in a turn fills what the first left empty
//!            (R1261). Indestructible (R46; never Taunt, R347)."

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-097-9";

/// §8.8: "Activate" is once a turn, the Radiant face's "Activate 2" twice.
const BASE_USES: i32 = 1;
const RADIANT_USES: i32 = 2;

/// §6.3 "Tribute 5": five of your Units, or Sheep Tokens worth as many (§3.2).
const TRIBUTE_COST: i32 = 5;

/// "Jlockheed Units": the Jlockeed-tagged Units of every set (R278, R380).
fn jlockeed_units() -> Value {
    json!({ "type": "Unit", "tags": ["Jlockeed"] })
}

fn fill(uses: i32, radiant: bool) -> ActivationDecl {
    ActivationDecl {
        id: "fill".to_string(),
        label: "Fill your board".to_string(),
        uses: ActivationUses::Count(uses),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(move |ctx| {
            fill_board_zones(ctx.state, ctx.controller)
                .iter()
                .map(|zone| {
                    summon_random(json_as(json!({
                        "query": jlockeed_units(),
                        "lane": zone.lane,
                        "radiant": radiant,
                    })))
                })
                .collect()
        }),
    }
}

fn headquarters(uses: i32, radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE_COST),
            ..StaticFlags::default()
        }),
        activations: vec![fill(uses, radiant)],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: headquarters(BASE_USES, false),
        radiant: headquarters(RADIANT_USES, true),
    }
}
