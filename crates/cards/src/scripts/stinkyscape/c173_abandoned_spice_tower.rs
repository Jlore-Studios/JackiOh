//! SPEC §8.10 Stinkyscape 173 Abandoned Spice Tower, a card of the Stinkyscape set.
//!   Base:    "Can't attack\nCan't be in Defense Position.\nDeath: Add {spices|Spice|Spices} to your
//!            hand.\nActivate: Take {selfDamage} damage. Tribute this." (0/8; 3, 4)
//!   Radiant: "… Death: Add {spices|Radiant Spice|Radiant Spices} to your hand.\nActivate: Take
//!            {selfDamage} damage. Tribute this." (0/16; 3, 4)
//!
//! R1840006's mining node, paying Spice: Can't attack is printed, and Can't Defend (R2657000) keeps
//! it out of Defense Position (§4.1). Activate (§6.2, R384), once a turn: its controller's hero takes
//! `selfDamage` (§4.4), then a Sacrifice of it (§6.3), a death, so its Death runs: `spices` fresh Spices
//! (S #89) to the hand, hand cards, never Resources, a full hand burning the rest (§2.4). Any other
//! death pays too. Radiant: Radiant Spices; the Activate keeps its damage, as the list prints it. The
//! numbers are declared (R386).

use jackioh_engine::effects::{add_to_hand, damage, sacrifice};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-173";

/// S #89, what its Death adds.
const SPICE: &str = "stinkyscape-089";
/// The Activate's id.
const MINE: &str = "mine";

/// "Activate: Take {selfDamage} damage. Tribute this.", on both faces.
fn mine() -> ActivationDecl {
    ActivationDecl {
        id: MINE.to_string(),
        label: "Take damage, then Tribute this".to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|ctx| {
            vec![
                damage(json_as(
                    json!({ "to": { "of": "selfHero" }, "amount": param(&*ctx, "selfDamage") }),
                )),
                sacrifice(json_as(json!({ "target": { "of": "self" } }))),
            ]
        }),
    }
}

fn face(radiant: bool) -> Script {
    Script {
        activations: vec![mine()],
        death: Some(hook(move |ctx| {
            (0..param(&*ctx, "spices"))
                .map(|_| add_to_hand(json_as(json!({ "defId": SPICE, "radiant": radiant }))))
                .collect()
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

/// R767000: through v0.5.9e "Can't be in Defense Position" was the static flag `never_defense` on both
/// faces, before R2657000 made it the keyword Can't Defend; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_9e() -> CardScripts {
    crate::never_defense_kept(script(), true, true)
}

// S 173 Abandoned Spice Tower — SPEC §8.10 row 173, BUILD M12 row S 173: "its Activate deals 4 to its
// controller's hero and tributes it, and its Death adds 3 Spices to the hand, a full hand burning the
// rest; it never attacks or switches to Defense; spices and selfDamage read through `param()`; radiant
// 0/16 and Radiant Spices, the damage kept".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::{ID, MINE, SPICE};
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";
    const BODY: &str = "core-019"; // 9/9

    fn tower(radiant: bool, hand: usize) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": filler(hand),
                "library": filler(4),
                "field": [{ "def": ID, "radiant": radiant }],
            },
            "p2": { "hand": [FILLER], "library": filler(4), "field": [BODY] },
        }))
    }

    fn spices(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1)
            .into_iter()
            .filter(|card| card.def_id == SPICE)
            .collect()
    }

    #[test]
    fn activate_damages_and_death_adds_spices() {
        let mut s = tower(false, 1);
        s.activate(ID, json!({ "ability": MINE }));
        s.expect_health(P1, 26);
        s.expect_in_zone(ID, "graveyard");
        let added = spices(&s);
        assert_eq!(added.len(), 3);
        assert!(added.iter().all(|card| !card.radiant));
        assert!(crate::resources_in_hand(&s, P1).is_empty(), "hand cards, never Resources");
    }

    #[test]
    fn a_full_hand_burns_the_rest() {
        let mut s = tower(false, HAND_CAP as usize - 1);
        s.activate(ID, json!({ "ability": MINE }));
        assert_eq!(spices(&s).len(), 1);
        let burned = s
            .pile(P1, "graveyard")
            .iter()
            .filter(|card| card.def_id == SPICE)
            .count();
        assert_eq!(burned, 2);
    }

    #[test]
    fn never_attacks_or_switches() {
        let mut s = tower(false, 1);
        s.expect_refused(|s| s.attack(ID, "hero"));
        s.expect_refused(|s| s.switch_position(ID));
        let target = s.unit_at(P2, 1);
        s.expect_refused(|s| s.attack(ID, &target));
    }

    #[test]
    fn spices_and_self_damage_read_through_param() {
        let mut s = tower(false, 1);
        set_param(s.card_mut(ID), "spices", 1);
        set_param(s.card_mut(ID), "selfDamage", 2);
        s.activate(ID, json!({ "ability": MINE }));
        s.expect_health(P1, 28);
        assert_eq!(spices(&s).len(), 1);
    }

    #[test]
    fn radiant_damage_kept_radiant_spices() {
        let mut s = tower(true, 1);
        s.activate(ID, json!({ "ability": MINE }));
        s.expect_health(P1, 26);
        let added = spices(&s);
        assert_eq!(added.len(), 3);
        assert!(added.iter().all(|card| card.radiant));
    }
}
