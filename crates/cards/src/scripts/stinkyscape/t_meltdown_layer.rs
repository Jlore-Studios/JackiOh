//! S T-meltdown-layer Drone Battleship (Meltdown Layer) (SPEC §8.10 row T-meltdown-layer): (5) Unit,
//! Token (printed Mythic), 12/12 → 24/24, a token of the Stinkyscape set.
//!   Both faces: "Starship, Charge, Windfury, First Strike, Brittle 3\nEnd of turn: This takes {selfDamage}
//!   damage." — selfDamage 6 (Brittle 6 on the Radiant face)
//! Engine: the bottom of the Drone Battleship's pile (S #36, R1842000): it acts once both layers above are
//!   gone, and with Charge may attack at once, twice (Windfury, §6.1), First Strike; all catalog keywords.
//!   Brittle 3 (R385): its count starts as it is summoned (R638) and ticks only on a pile's top
//!   (`brittle::ticked_cards`), so it waits while the layer is buried. End of turn (R62): one hit of
//!   `selfDamage` on itself (§4.4), never its hero, its own hook, which a buried card does not run (R13). A
//!   unit token: it ceases to exist off the field (R11).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-meltdown-layer";

fn meltdown() -> Script {
    Script {
        end_of_turn: Some(hook(|ctx| {
            vec![damage(json_as(json!({
                "to": { "of": "self" },
                "amount": param(&*ctx, "selfDamage"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // One script: the Radiant face's 24/24 and Brittle 6 are catalog data.
    CardScripts {
        base: meltdown(),
        radiant: meltdown(),
    }
}

// S T-meltdown-layer — SPEC §8.10 row T-meltdown-layer, BUILD M12 row S T-meltdown-layer.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MELTDOWN: &str = "stinkyscape-t-meltdown-layer";
    const CORE_X: &str = "stinkyscape-036";
    const HIT_JOB: &str = "core-016"; // (3) Spell: destroy a Unit.
    const STRIKER: &str = "classic-049"; // (3) 9/9 Rush.
    const FILLER: &str = "core-005";

    fn alone(radiant: bool) -> (Scenario, String) {
        let s = scenario(json!({
            "p1": { "hand": [FILLER], "field": [{ "def": MELTDOWN, "radiant": radiant }], "library": filler(6) },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }));
        let id = s.unit(P1, 1).expect("the Meltdown Layer").id;
        (s, id)
    }

    fn battleship(p2: Value) -> (Scenario, String) {
        let mut p2 = p2;
        p2["library"] = filler(6);
        let mut s = scenario(json!({
            "p1": { "hand": [CORE_X, HIT_JOB, HIT_JOB, FILLER], "library": filler(6), "mana": 40 },
            "p2": p2,
        }));
        s.play(CORE_X, json!({}));
        let meltdown = pile_at(s.state(), ZoneSlot { player: P1, row: Row::Units, lane: 1 })
            .and_then(|pile| pile.last().cloned())
            .expect("the pile's bottom");
        assert_eq!(meltdown.def_id, MELTDOWN);
        (s, meltdown.id)
    }

    fn clear_top(s: &mut Scenario) {
        let top = s.unit(P1, 1).expect("the pile's top").id;
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": top }] }));
    }

    fn brittle(s: &Scenario, id: &str) -> Option<i32> {
        active_brittle_count(s.card(id))
    }

    #[test]
    fn resumed_it_attacks_twice_the_turn_it_lands_striking_first() {
        let (mut s, meltdown) = battleship(json!({ "hand": [FILLER], "field": [STRIKER] }));
        clear_top(&mut s);
        clear_top(&mut s);
        assert_eq!(s.unit(P1, 1).expect("the top").id, meltdown, "both layers above are gone");
        s.attack(meltdown.as_str(), STRIKER);
        s.expect_in_zone(STRIKER, "graveyard");
        assert_eq!(s.card(meltdown.as_str()).damage, 0);
        s.attack(meltdown.as_str(), "hero");
        s.expect_health(P2, HERO_HEALTH - 12);
        s.expect_refused(|s| s.attack(meltdown.as_str(), "hero"));
    }

    #[test]
    fn it_takes_6_at_its_controllers_end_of_turn_and_dies_in_two() {
        let (mut s, id) = alone(false);
        s.end_turn();
        assert_eq!(s.card(id.as_str()).damage, 6, "its controller's end of turn");
        s.expect_health(P1, HERO_HEALTH);
        s.end_turn();
        assert_eq!(s.card(id.as_str()).damage, 6, "the opponent's end of turn hits nothing");
        s.end_turn();
        s.expect_in_zone(id.as_str(), "gone");
    }

    #[test]
    fn r13_buried_it_takes_nothing_and_its_brittle_waits() {
        let (mut s, meltdown) = battleship(json!({ "hand": [FILLER] }));
        assert_eq!(brittle(&s, &meltdown), Some(3), "the count starts as it is summoned");
        s.end_turn();
        assert_eq!(s.card(meltdown.as_str()).damage, 0, "buried, no end of turn");
        s.end_turn();
        assert_eq!(brittle(&s, &meltdown), Some(3), "buried, no tick");
        s.state_mut().players.p1.mana.current = 10;
        clear_top(&mut s);
        clear_top(&mut s);
        s.end_turn();
        assert_eq!(s.card(meltdown.as_str()).damage, 6, "on top, its end of turn runs");
        s.end_turn();
        assert_eq!(brittle(&s, &meltdown), Some(2), "on top, it ticks");
    }

    #[test]
    fn selfdamage_reads_through_param() {
        let (mut s, id) = alone(false);
        set_param(s.card_mut(id.as_str()), "selfDamage", 2);
        s.end_turn();
        assert_eq!(s.card(id.as_str()).damage, 2);
    }

    #[test]
    fn radiant_24_24_brittle_6() {
        let (mut s, id) = alone(true);
        let stats = s.stats(id.as_str());
        assert_eq!((stats.attack, stats.health), (24, 24));
        assert_eq!(brittle(&s, &id), Some(6));
        s.end_turn();
        assert_eq!(s.card(id.as_str()).damage, 6);
    }
}
