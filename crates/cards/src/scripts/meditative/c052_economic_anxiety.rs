//! Meditative #52, Economic Anxiety (docs/meditative-set.md M6): a Field Spell whose aura is not a
//! layer — a combat-only attack modifier (MD-D4, R1120).
//!
//! Base: Units have +{bonus} Attack while attacking a Unit that shares none of their tags. Radiant:
//! your Units have +{bonus} Attack and Poisonous while doing so. Tags are printed (`def_of(..).tags`),
//! so granted tags never count; an Untagged Unit shares nothing, so it qualifies on both sides (M10).
//! The Radiant face covers only its controller's attackers. `bonus` reads through `param()`, so
//! tuning moves it (M10). The bonus reads the defender's tags, so it never applies against a Unit
//! immune to tribal tag based hate (M #25), whatever the attacker's tags (MD-B1, R940).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-052";

fn economic_anxiety(yours_only: bool, poisonous: bool) -> Script {
    Script {
        attack_mods: Some(attack_mod_hook(move |args| {
            if yours_only && args.attacker.controller != args.self_.controller {
                return AttackMod::default();
            }
            // MD-B1, R940: M #25 is immune to it, an untagged attacker included; the view's
            // `conditionTargets` reads this same hook, so the yellow skips it too.
            if immune_to_tribal_hate(args.state, args.defender) {
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
    /// M #25 Blue-Eyes White Felinor, 12/9 Felinor, immune to tribal tag based hate.
    const WHITE_FELINOR: &str = "meditative-025";
    /// C #61 Plague Bringer Goliath 7/7 Plague, Trample; C #73 Nurse Cleaver 3/6 untagged, Cleave.
    const GOLIATH: &str = "classic-061";
    const CLEAVER: &str = "classic-073";

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

    /// R940's game: a Human and an untagged Scarab behind Economic Anxiety, facing M #25 (lane 1)
    /// and a big Felinor (lane 2) that holds no immunity. Neither attacker shares a tag with either.
    fn white_felinor_game(radiant: bool) -> Scenario {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [VANILLA, SCARAB], "backrow": [ID] },
            "p2": { "field": [WHITE_FELINOR, BIG] },
        }));
        if radiant {
            make_radiant(&mut s, ID);
        }
        s
    }

    /// The yellow on `attacker` in its controller's view (`conditionTargets`, R1121), empty for none.
    fn yellow(s: &Scenario, attacker: &str) -> Vec<String> {
        let view = s.view(P1);
        let unit = view
            .you
            .units
            .iter()
            .flatten()
            .find(|unit| unit.instance_id == attacker);
        unit.and_then(|unit| unit.condition_targets.clone())
            .unwrap_or_default()
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
    fn r1120_trample_excess_carries_the_bonus() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [GOLIATH], "backrow": [ID] },
            "p2": { "field": [VANILLA] },
        }));
        let goliath = s.unit(P1, 1).unwrap().id;
        let victim = s.unit(P2, 1).unwrap().id;
        s.attack(&goliath, &victim);
        // 7 + 3 into a 4/4 Human: 6 tramples over to the hero, not 3.
        s.expect_in_zone(&victim, "graveyard");
        assert_eq!(dealt(&s, &goliath, "hero-p2"), Some(6));
    }

    #[test]
    fn r1120_a_cleave_splash_carries_the_bonus() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [{ "def": CLEAVER, "lane": 2 }], "backrow": [ID] },
            "p2": { "field": [{ "def": BIG, "lane": 1 }, { "def": VANILLA, "lane": 2 }] },
        }));
        let cleaver = s.unit(P1, 2).unwrap().id;
        let victim = s.unit(P2, 2).unwrap().id;
        let beside = s.unit(P2, 1).unwrap().id;
        s.attack(&cleaver, &victim);
        // The untagged Cleaver's strike and its splash are both 3 + 3.
        assert_eq!(dealt(&s, &cleaver, &victim), Some(6));
        assert_eq!(dealt(&s, &cleaver, &beside), Some(6));
    }

    #[test]
    fn r1120_the_base_face_bonuses_the_opponents_units_too() {
        let mut s = anxiety_game();
        s.end_turn();
        // Their Human into our Felinor shares no tag: 4 + 3.
        let vanilla = s.unit(P2, 1).unwrap().id;
        let ours = s.unit(P1, 1).unwrap().id;
        s.attack(&vanilla, &ours);
        assert_eq!(dealt(&s, &vanilla, &ours), Some(7));
    }

    #[test]
    fn r1120_an_attack_on_a_hero_gets_nothing() {
        let mut s = anxiety_game();
        let felinor = s.unit(P1, 1).unwrap().id;
        s.attack(&felinor, "hero");
        s.expect_health(P2, 30 - 3);
    }

    #[test]
    fn r1120_the_displayed_attack_never_changes() {
        crate::register_all();
        // A big Felinor (3/10) into a Human 4/4: it survives the strike back.
        let mut s = scenario(json!({
            "p1": { "field": [BIG], "backrow": [ID] },
            "p2": { "field": [VANILLA] },
        }));
        let felinor = s.unit(P1, 1).unwrap().id;
        let unit_attack = |s: &Scenario, id: &str| {
            s.view(P1)
                .you
                .units
                .iter()
                .flatten()
                .find(|unit| unit.instance_id == id)
                .map(|unit| unit.attack)
        };
        assert_eq!(shown(&s, &felinor), 3);
        assert_eq!(unit_attack(&s, &felinor), Some(3));
        let foe = s.unit(P2, 1).unwrap().id;
        s.attack(&felinor, &foe);
        assert_eq!(dealt(&s, &felinor, &foe), Some(6));
        // Shown 3 before and after the bonused strike.
        assert_eq!(shown(&s, &felinor), 3);
        assert_eq!(unit_attack(&s, &felinor), Some(3));
    }

    #[test]
    fn r1121_condition_targets_are_on_the_viewers_own_attackers_in_their_main_phase() {
        let mut s = anxiety_game();
        let felinor = s.unit(P1, 1).unwrap().id;
        let scarab = s.unit(P1, 2).unwrap().id;
        let vanilla = s.unit(P2, 1).unwrap().id;
        let big = s.unit(P2, 2).unwrap().id;
        // The Felinor shares Felinor with the big one: only the Human qualifies. The untagged
        // Scarab qualifies against both.
        assert_eq!(yellow(&s, &felinor), vec![vanilla.clone()]);
        assert_eq!(yellow(&s, &scarab), vec![vanilla.clone(), big.clone()]);
        // Never on enemy Units, in either seat's view, on p1's turn.
        let theirs = s.view(P1);
        assert!(
            theirs
                .opponent
                .units
                .iter()
                .flatten()
                .all(|unit| unit.condition_targets.is_none())
        );
        let foe = s.view(P2);
        assert!(
            foe.you
                .units
                .iter()
                .flatten()
                .all(|unit| unit.condition_targets.is_none())
        );
        // Absent on the opponent's turn.
        s.end_turn();
        assert!(yellow(&s, &felinor).is_empty());
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
    fn r940_no_bonus_against_white_felinor() {
        let mut s = white_felinor_game(false);
        let vanilla = s.unit(P1, 1).unwrap().id;
        let scarab = s.unit(P1, 2).unwrap().id;
        let white = s.unit(P2, 1).unwrap().id;
        let big = s.unit(P2, 2).unwrap().id;
        // The yellow lists the big Felinor alone, on the tagged attacker and the untagged one.
        assert_eq!(yellow(&s, &vanilla), vec![big.clone()]);
        assert_eq!(yellow(&s, &scarab), vec![big.clone()]);
        // The Human into M #25 deals its printed 4; the untagged Scarab its printed 1.
        s.attack(&vanilla, &white);
        assert_eq!(dealt(&s, &vanilla, &white), Some(4));
        s.attack(&scarab, &white);
        assert_eq!(dealt(&s, &scarab, &white), Some(1));
        assert_eq!(s.card(&white).damage, 5);
        // The same Human into the big Felinor still gets the bonus: 4 + 3.
        let mut s = white_felinor_game(false);
        let (vanilla, big) = (s.unit(P1, 1).unwrap().id, s.unit(P2, 2).unwrap().id);
        s.attack(&vanilla, &big);
        assert_eq!(dealt(&s, &vanilla, &big), Some(7));
    }

    #[test]
    fn r940_radiant_no_bonus_nor_poisonous_against_white_felinor() {
        let mut s = white_felinor_game(true);
        let vanilla = s.unit(P1, 1).unwrap().id;
        let white = s.unit(P2, 1).unwrap().id;
        let big = s.unit(P2, 2).unwrap().id;
        assert_eq!(yellow(&s, &vanilla), vec![big.clone()]);
        // Into M #25: the printed 4 and no Poisonous, so the 12/9 stands.
        s.attack(&vanilla, &white);
        assert_eq!(dealt(&s, &vanilla, &white), Some(4));
        s.expect_in_zone(&white, "field");
        assert_eq!(s.card(&white).damage, 4);
        // The same Human into the big Felinor: 4 + 3 and Poisonous, which destroys the 3/10.
        let mut s = white_felinor_game(true);
        let (vanilla, big) = (s.unit(P1, 1).unwrap().id, s.unit(P2, 2).unwrap().id);
        s.attack(&vanilla, &big);
        assert_eq!(dealt(&s, &vanilla, &big), Some(7));
        s.expect_in_zone(&big, "graveyard");
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
