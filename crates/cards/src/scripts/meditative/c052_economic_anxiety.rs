//! Meditative #52, Economic Anxiety (docs/meditative-set.md M6): a Field Spell whose aura is not a
//! layer — a combat-only attack modifier (MD-D4, R1120).
//!
//! Base: Units have +{bonus} Attack while attacking a Unit that shares none of their tags. Radiant:
//! your Units have +{bonus} Attack and Poisonous while doing so. Tags are printed (`def_of(..).tags`),
//! so granted tags never count; an Untagged Unit shares nothing, so it qualifies on both sides (M10).
//! The Radiant face covers only its controller's attackers. `bonus` reads through `param()`, so
//! tuning moves it (M10).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-052";

fn economic_anxiety(yours_only: bool, poisonous: bool) -> Script {
    Script {
        attack_mods: Some(attack_mod_hook(move |args| {
            if yours_only && args.attacker.controller != args.self_.controller {
                return AttackMod::default();
            }
            let attacker_tags = &def_of(Some(args.state), &args.attacker.def_id).tags;
            let defender_tags = &def_of(Some(args.state), &args.defender.def_id).tags;
            if attacker_tags.iter().any(|tag| defender_tags.contains(tag)) {
                return AttackMod::default();
            }
            AttackMod {
                attack: param(&args, "bonus"),
                poisonous,
            }
        })),
        ..Script::default()
    }
}

pub fn base() -> Script {
    economic_anxiety(false, false)
}

pub fn radiant() -> Script {
    economic_anxiety(true, true)
}

pub fn script() -> CardScripts {
    CardScripts {
        base: economic_anxiety(false, false),
        radiant: economic_anxiety(true, true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// Felinor 3/4, Human 4/4, big Felinor 3/10, untagged Scarab 1/1.
    const FELINOR: &str = "core-012";
    const VANILLA: &str = "core-008";
    const BIG: &str = "core-043";
    const SCARAB: &str = "core-007";

    /// `SideSetup.backrow` takes no `radiant` flag: flip the instance directly.
    fn make_radiant(s: &mut Scenario, card: &str) {
        let id = s.card(card).id.clone();
        match find_instance_mut(s.state_mut(), &id) {
            Some(instance) => instance.radiant = true,
            None => panic!("no instance {id}"),
        }
    }

    /// The M10 row's game: Economic Anxiety behind two Units, facing two enemies.
    fn anxiety_game() -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": { "field": [FELINOR, SCARAB], "backrow": [ID] },
            "p2": { "field": [VANILLA, BIG] },
        }))
    }

    /// The listed attack, as the engine layers it.
    fn shown(s: &Scenario, card: &str) -> i32 {
        s.stats(card).attack
    }

    /// What the last step's hit from `source` on `target` dealt: its `damage` event's amount. A unit
    /// the hit destroyed lies in the graveyard with its damage gone, so the event is the record.
    fn dealt(s: &Scenario, source: &str, target: &str) -> Option<i32> {
        s.last_events().iter().find_map(|event| match event {
            GameEvent::Damage {
                source_id,
                target_id,
                amount,
                ..
            } if source_id.as_deref() == Some(source) && target_id == target => Some(*amount),
            _ => None,
        })
    }

    #[test]
    fn unshared_tags_bonus() {
        let mut s = anxiety_game();
        // Felinor into Vanilla (Felinor vs Human): 3 shown, 3 + 3 dealt.
        let attacker = s.unit(P1, 1).unwrap().id;
        let victim = s.unit(P2, 1).unwrap().id;
        assert_eq!(shown(&s, &attacker), 3);
        s.attack(&attacker, &victim);
        assert_eq!(dealt(&s, &attacker, &victim), Some(6));
        s.expect_in_zone(&victim, "graveyard");
        // The strike back is the Vanilla's own 4, never bonused.
        assert_eq!(dealt(&s, &victim, &attacker), Some(4));
    }

    #[test]
    fn shared_tag_no_bonus() {
        let mut s = anxiety_game();
        // Felinor into big Felinor (Felinor vs Felinor): 3 shown, 3 dealt.
        let foe = s.unit(P2, 2).unwrap().id;
        s.attack(&s.unit(P1, 1).unwrap().id, &foe);
        assert_eq!(s.card(&foe).damage, 3);
        s.expect_in_zone(&foe, "field");
    }

    #[test]
    fn untagged_qualifies() {
        let mut s = anxiety_game();
        // The untagged Scarab shares nothing, so it qualifies even against a tagged foe: 1 + 3.
        let scarab = s.unit(P1, 2).unwrap().id;
        let foe = s.unit(P2, 1).unwrap().id;
        s.attack(&scarab, &foe);
        assert_eq!(dealt(&s, &scarab, &foe), Some(4));
        s.expect_in_zone(&foe, "graveyard");
    }

    #[test]
    fn radiant_yours_only_poisonous() {
        let mut s = anxiety_game();
        make_radiant(&mut s, ID);
        // The opponent's attacker gets nothing from the Radiant face: their Human into our Felinor
        // shares no tag, which the base face would bonus, yet deals its printed 4.
        s.end_turn();
        let vanilla = s.unit(P2, 1).unwrap().id;
        let ours = s.unit(P1, 1).unwrap().id;
        s.attack(&vanilla, &ours);
        assert_eq!(dealt(&s, &vanilla, &ours), Some(4));
        // Ours gets the bonus and the Poisonous mark with it: the Scarab's 1 + 3 destroys a 3/10.
        s.end_turn();
        let scarab = s.unit(P1, 2).unwrap().id;
        let big_foe = s.unit(P2, 2).unwrap().id;
        s.attack(&scarab, &big_foe);
        assert_eq!(dealt(&s, &scarab, &big_foe), Some(4));
        s.expect_in_zone(&big_foe, "graveyard");
    }

    #[test]
    fn bonus_tunes() {
        let mut s = anxiety_game();
        set_param(s.card_mut(ID), "bonus", 1);
        // The untagged Scarab into big Felinor: 1 + 1 dealt, which the 3/10 survives.
        let scarab = s.unit(P1, 2).unwrap().id;
        let foe = s.unit(P2, 2).unwrap().id;
        s.attack(&scarab, &foe);
        assert_eq!(dealt(&s, &scarab, &foe), Some(2));
        assert_eq!(s.card(&foe).damage, 2);
    }
}
