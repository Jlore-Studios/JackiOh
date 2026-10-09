//! M #95.1 CN Golem (SPEC §8.8 row 95.1, §7; R42, R80, R311, R1245): (4) Unit, CN, Token, printed
//! Legendary, 10/10 → 20/20.
//!
//! Base:    "Rush, Poisonous, Cleave, Pierce\nWhenever this destroys a Unit, shuffle
//!          {viruses|CN-Virus|CN-Viruses} into your opponent's deck."
//! Radiant: "Rush, Poisonous, Cleave, Pierce, Windfury\nWhenever this destroys a Unit, shuffle
//!          {viruses|Radiant CN-Virus|Radiant CN-Viruses} into your opponent's deck."
//! Engine: the keywords are catalog data (§6.1). A kill trigger, C+ #19.5 Bot Loser's: on each
//! `destroyed` whose `killerId` is this (R42), Cleave and Poisonous kills included, while it still
//! stands on the field as the event is dispatched (R1245), `shuffle_into` the opponent's deck
//! `viruses` CN-Viruses (Core #90.1), the opponent's cards, Radiant on the Radiant face; the shuffle-in
//! is public and recorded in their list (R311), and a full deck refuses it (R80).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-095-1";

/// §7: the CN-Virus it shuffles in.
const CN_VIRUS: &str = "core-090-1";

/// "Whenever this destroys a Unit, shuffle {viruses} CN-Virus into your opponent's deck."
fn on_kill(radiant: bool) -> TriggerDef {
    TriggerDef::new("cn-golem-kill", &[GameEventType::Destroyed], move |ctx, event| {
        let killed_by_me = match (event, ctx.self_.as_ref()) {
            (
                GameEvent::Destroyed {
                    killer_id: Some(killer),
                    ..
                },
                Some(me),
            ) => *killer == me.id,
            _ => false,
        };
        if killed_by_me {
            vec![shuffle_into(json_as(json!({
                "defId": CN_VIRUS,
                "count": param(&*ctx, "viruses"),
                "player": "enemy",
                "radiant": radiant,
            })))]
        } else {
            Vec::new()
        }
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        triggers: vec![on_kill(false)],
        ..Script::default()
    };
    let radiant = Script {
        triggers: vec![on_kill(true)],
        ..Script::default()
    };
    CardScripts { base, radiant }
}
