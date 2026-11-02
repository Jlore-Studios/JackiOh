//! S #37 Kwzka (SPEC §8.10 row 37): (3) Unit, Human, Acclaimed, Legendary, 6/9 → 12/18, of the
//! Stinkyscape set.
//!   Base:    "Aura: Each enemy Unit played that costs less than this is Allured. / Death: Remove every
//!            Allure on the field."
//!   Radiant: "Divine Shield / Aura: …" the same text.
//! Engine: while it acts on the field, each Unit its controller's opponent plays (a play, R70: a summon is
//! none) whose cost paid (R56, the `cardPlayed` event's) is less than this card's cost now (R66) is
//! Allured as it arrives (R963, R1045000): the engine's delayed steal at the start of this card's
//! controller's next turn, destroyed with no room, aimed at its stay and carrying the public pink
//! `allure` mark while it waits (R1876004). Death (§6.2): `remove_allures` (R1876002): every Allure mark
//! on the field, both sides' and whoever made it, comes off and its steal is canceled. Divine Shield on
//! the Radiant face is catalog data.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-037";

/// The step the engine's delayed Allure stores (`DELAYED_ALLURE_HOOK` resolves it by its data).
const ALLURE_STEP: &str = "unit";

/// R1045000, R963: Allure one enemy Unit — stolen at the start of this player's next turn, destroyed
/// with no open zone in its row, aimed at its stay on the field and marked pink while it waits.
fn allure(instance_id: &str) -> Effect {
    delay(json_as(json!({
        "at": { "phase": "start", "player": "self" },
        "step": ALLURE_STEP,
        "hook": DELAYED_ALLURE_HOOK,
        "data": { "instanceId": instance_id },
        "watch": instance_id,
        "mark": { "mark": ALLURE_MARK, "color": ALLURE_MARK_COLOR },
    })))
}

/// R1876004: the enemy Unit a `cardPlayed` names, still on the field, played for less than this card
/// costs now.
fn cheaper_enemy_unit(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<String> {
    let GameEvent::CardPlayed {
        player,
        instance_id,
        cost_paid,
        ..
    } = event
    else {
        return None;
    };
    let me = ctx.self_.as_ref()?;
    if *player == ctx.controller {
        return None;
    }
    let played = find_instance(ctx.state, instance_id)?;
    if card_type_of(ctx.state, played) != CardType::Unit || !acts_on_field(ctx.state, played) {
        return None;
    }
    (*cost_paid < cost_now(ctx.state, me)).then(|| played.id.clone())
}

pub fn script() -> CardScripts {
    let base = Script {
        triggers: vec![
            TriggerDef::new("allure", &[GameEventType::CardPlayed], |ctx, event| {
                cheaper_enemy_unit(ctx, event)
                    .map(|id| vec![allure(&id)])
                    .unwrap_or_default()
            })
            .with_when(|ctx, event| cheaper_enemy_unit(ctx, event).is_some()),
        ],
        death: Some(hook(|_ctx| vec![remove_allures()])),
        ..Script::default()
    };
    // The Radiant face adds Divine Shield, catalog data.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// S #37 Kwzka — SPEC §8.10 row 37, BUILD M12 row S 37 (R62, R82, R963).
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::ALLURE_MARK;
    use jackioh_engine::marks::marks_on;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const VANILLA: &str = "core-008"; // (1) 4/4.
    const MENACE: &str = "core-019"; // (3) 9/9.
    const MR_TOKEN: &str = "core-015"; // (1) 1/1: Cry: summon a Rush Token.
    const FANATIC: &str = "core-050"; // (2) 1/1: Cry: Allure an enemy permanent.
    const FILLER: &str = "core-005";

    fn board(radiant: bool, p2_hand: Value, p2_field: Value) -> Scenario {
        let mut s = scenario(json!({
            "active": "p2",
            "p1": {
                "hand": [FILLER],
                "field": [{ "def": ID, "radiant": radiant }, VANILLA],
                "library": filler(6),
            },
            "p2": { "hand": p2_hand, "field": p2_field, "library": filler(6), "mana": 10 },
        }));
        s.state_mut().players.p2.auto_end_turn = Some(false);
        s
    }

    fn allured(s: &Scenario, card: &CardInstance) -> bool {
        marks_on(s.state(), &card.id).iter().any(|mark| mark.mark == ALLURE_MARK)
    }

    #[test]
    fn r1876004_a_cheaper_enemy_unit_played_is_allured_and_stolen_at_your_next_turn() {
        let mut s = board(false, json!([VANILLA, FILLER]), json!([]));
        s.play(VANILLA, json!({ "zone": 1 }));
        let theirs = s.unit_at(P2, 1);
        assert!(allured(&s, &theirs));
        s.end_turn();
        assert_eq!(s.card(&theirs).controller, P1);
        assert!(!allured(&s, &theirs));
    }

    #[test]
    fn r1876004_one_at_its_cost_or_above_is_not() {
        let mut s = board(false, json!([MENACE, FILLER]), json!([]));
        s.play(MENACE, json!({ "zone": 1 }));
        let theirs = s.unit_at(P2, 1);
        assert!(!allured(&s, &theirs));
        s.end_turn();
        assert_eq!(s.card(&theirs).controller, P2);
    }

    #[test]
    fn r1876004_a_summoned_unit_is_no_play_and_is_not_allured() {
        let mut s = board(false, json!([MR_TOKEN, FILLER]), json!([]));
        s.play(MR_TOKEN, json!({ "zone": 1 }));
        let played = s.unit_at(P2, 1);
        let token = s.unit_at(P2, 2);
        assert!(allured(&s, &played));
        assert!(!allured(&s, &token));
    }

    #[test]
    fn r1876004_your_own_plays_are_never_allured() {
        let mut s = scenario(json!({
            "p1": { "hand": [VANILLA, FILLER], "field": [ID], "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(VANILLA, json!({ "zone": 2 }));
        assert!(!allured(&s, &s.unit_at(P1, 2)));
    }

    #[test]
    fn r1876002_its_death_removes_every_allure_on_the_field_so_nothing_is_stolen() {
        let mut s = board(false, json!([FANATIC, FILLER]), json!([{ "def": MENACE, "lane": 5 }]));
        let vanilla = s.unit_at(P1, 2);
        s.play(FANATIC, json!({ "zone": 1, "targets": [{ "pick": "instance", "instanceId": vanilla.id }] }));
        let fanatic = s.unit_at(P2, 1);
        assert!(allured(&s, &fanatic) && allured(&s, &vanilla));
        let kwzka = s.unit_at(P1, 1);
        s.attack(s.unit_at(P2, 5), &kwzka);
        assert!(s.unit(P1, 1).is_none(), "Kwzka died");
        assert!(!allured(&s, &fanatic) && !allured(&s, &vanilla));
        s.end_turn();
        assert_eq!(s.card(&fanatic).controller, P2, "Kwzka's steal canceled");
        s.end_turn();
        assert_eq!(s.card(&vanilla).controller, P1, "the Fanatic's steal canceled too");
    }

    #[test]
    fn r275_radiant_12_18_with_divine_shield() {
        let mut s = board(true, json!([VANILLA, FILLER]), json!([]));
        let kwzka = s.unit_at(P1, 1);
        assert!(unit_has(s.state(), &kwzka, KeywordKind::DivineShield));
        s.expect_stats(&kwzka, json!({ "attack": 12, "health": 18 }));
        s.play(VANILLA, json!({ "zone": 1 }));
        assert!(allured(&s, &s.unit_at(P2, 1)));
    }
}
